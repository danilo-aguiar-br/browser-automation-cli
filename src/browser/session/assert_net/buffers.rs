// SPDX-License-Identifier: MIT OR Apache-2.0
//! Buffer composition and paging shared by the `console` and `net` readers.
//!
//! # Why these two live here and the filter does NOT
//!
//! `console_list` and `net_list` are siblings: both gate on a capture flag,
//! compose a view over preserved rings plus the live log, filter, and page. Two
//! of those steps are the SAME logic over a different pair of fields, and two
//! are deliberately different.
//!
//! The shared ones are here because these two readers have already drifted
//! apart once. `net get 0` and `console get 0` used to address different
//! records than the `list` beside them — the H5 and H6 findings of this
//! release — precisely because the same idea was written twice and corrected
//! once. Logic that must agree is safer as one function than as two functions
//! and a habit of remembering.
//!
//! The FILTER stays duplicated on purpose. `net` matches a closed CDP
//! vocabulary by case-insensitive equality and REFUSES a token outside it;
//! `console` matches free-form CSV by substring. Folding those together would
//! erase the fail-closed refusal this release exists to add. The capture gate
//! stays too: it is seven lines naming a different flag and a different
//! remedy, and sharing it would buy nothing but an argument.

use serde_json::Value;

/// Compose the record view a `list` answers over, with the mode that describes it.
///
/// `include_preserved` widens the view from the current navigation to every
/// retained ring. The returned label is what the envelope reports, so the
/// caller never has to restate which of the three cases it is in.
///
/// # Why this materializes instead of returning an iterator
///
/// An audit asked for paging over an iterator so a `list` would not build the
/// whole view to answer one page. That item was RETIRED rather than done, and
/// the reason belongs here so it is not re-opened on sight.
///
/// The cost it targeted no longer exists. When it was written the live buffers
/// grew without a ceiling, so "materialize everything" had no upper bound; the
/// same wave gave them the `event_tracker_max_entries` ring, which bounds this
/// clone at a knob-controlled number of records in a one-shot process that
/// exits immediately after.
///
/// What an iterator would cost is concrete. Every caller needs the CARDINALITY
/// of the view — `total` is the paging denominator — and `net_get` needs the
/// whole composed sequence to address an id, so the sequence would be walked
/// twice or collected anyway. Returning borrowed items from two different
/// owners would put lifetimes on a signature whose entire purpose is that
/// `list` and `get` cannot disagree about the order they see. That agreement
/// is the H5/H6 defect this function exists to prevent, and it is worth more
/// than the allocation.
pub(super) fn compose_view(
    preserved: &[Vec<Value>],
    live: &[Value],
    include_preserved: bool,
) -> (Vec<Value>, &'static str) {
    if !include_preserved {
        return (live.to_vec(), "current_navigation");
    }
    let mut out: Vec<Value> =
        Vec::with_capacity(preserved.iter().map(Vec::len).sum::<usize>() + live.len());
    for ring in preserved {
        out.extend(ring.iter().cloned());
    }
    out.extend(live.iter().cloned());
    let mode = if preserved.is_empty() {
        "process_local_only"
    } else {
        "preserved_ring"
    };
    (out, mode)
}

/// One page of a list: the slice to cut, and the numbers the envelope reports.
///
/// # Why all four and not just the slice
///
/// The first shape of [`page_bounds`] returned only `(start, end)`, which is
/// what the SLICING needs. It compiled nowhere: both callers also report
/// `page_idx` and `page_size` back in the envelope, so the resolved values are
/// an output of this computation and not scratch space inside it. Returning
/// them keeps the resolution in ONE place — a caller that re-derived `size`
/// with its own `unwrap_or` could disagree with the slice it was handed.
#[derive(Clone, Copy)]
pub(super) struct Page {
    /// Zero-based page the caller asked for, defaulted.
    pub index: usize,
    /// Records per page, defaulted to the whole set.
    pub size: usize,
    /// First index of the slice, clamped to `total`.
    pub start: usize,
    /// One past the last index, clamped to `total`.
    pub end: usize,
}

/// Resolve the `[start, end)` slice one page covers, clamped to `total`.
///
/// A `page_size` of `None` means "the whole set", which is why the default is
/// `total` rather than a number: a reader who named no page wants all of it.
/// `total.max(1)` keeps the size non-zero on an empty set, so the arithmetic
/// below cannot divide the caller into an empty page they did not ask for.
pub(super) fn page_bounds(total: usize, page_idx: Option<usize>, page_size: Option<usize>) -> Page {
    let index = page_idx.unwrap_or(0);
    let size = page_size.unwrap_or(total.max(1));
    let start = index.saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    Page {
        index,
        size,
        start,
        end,
    }
}

#[cfg(test)]
mod tests {
    use super::{compose_view, page_bounds, Page};
    use serde_json::json;

    /// The slice half of a [`Page`], which is what most of these assert on.
    fn slice(p: Page) -> (usize, usize) {
        (p.start, p.end)
    }

    #[test]
    fn without_preserved_only_the_live_log_is_seen() {
        let live = vec![json!({"id": 1})];
        let (view, mode) = compose_view(&[vec![json!({"id": 0})]], &live, false);
        assert_eq!(view, live);
        assert_eq!(mode, "current_navigation");
    }

    /// Preserved rings come BEFORE the live log, oldest first.
    ///
    /// This order is what `get` indexes against. When the two readers disagreed
    /// about it, index 0 of `list` and `get 0` returned different records while
    /// both reported success.
    #[test]
    fn preserved_rings_precede_the_live_log_in_order() {
        let preserved = vec![vec![json!({"id": 0})], vec![json!({"id": 1})]];
        let live = vec![json!({"id": 2})];
        let (view, mode) = compose_view(&preserved, &live, true);
        assert_eq!(
            view,
            vec![json!({"id": 0}), json!({"id": 1}), json!({"id": 2})]
        );
        assert_eq!(mode, "preserved_ring");
    }

    #[test]
    fn asking_for_preserved_when_there_are_none_says_so() {
        let live = vec![json!({"id": 1})];
        let (view, mode) = compose_view(&[], &live, true);
        assert_eq!(view, live);
        assert_eq!(mode, "process_local_only");
    }

    #[test]
    fn no_page_size_covers_the_whole_set() {
        let p = page_bounds(25, None, None);
        assert_eq!((p.start, p.end), (0, 25));
        assert_eq!((p.index, p.size), (0, 25));
    }

    #[test]
    fn pages_partition_the_set_without_gap_or_overlap() {
        assert_eq!(slice(page_bounds(25, Some(0), Some(10))), (0, 10));
        assert_eq!(slice(page_bounds(25, Some(1), Some(10))), (10, 20));
        assert_eq!(slice(page_bounds(25, Some(2), Some(10))), (20, 25));
    }

    /// A page past the end is empty, never a panic and never a wrapped index.
    #[test]
    fn a_page_beyond_the_end_is_empty() {
        assert_eq!(slice(page_bounds(25, Some(99), Some(10))), (25, 25));
        assert_eq!(slice(page_bounds(0, Some(0), Some(10))), (0, 0));
    }

    /// Overflow cannot turn a far page into a valid slice.
    #[test]
    fn an_absurd_page_index_saturates_instead_of_wrapping() {
        assert_eq!(slice(page_bounds(25, Some(usize::MAX), Some(10))), (25, 25));
    }
}
