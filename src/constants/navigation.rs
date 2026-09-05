// SPDX-License-Identifier: MIT OR Apache-2.0
//! Navigation targets this product names in more than one place.

/// The blank page a session opens before it is told where to go.
///
/// # Why this is a constant and not a bare literal
///
/// Not for tidiness, and not because the value could change — it is a web
/// standard and it will not. It is here because of the COMPARISONS. This string
/// was written 50 times across 28 files, and 14 of those occurrences were
/// equality tests deciding whether a page had been navigated yet.
///
/// A typo in a comparison against a string literal compiles cleanly and then
/// never matches. `url != "about:blnk"` is always true, the guard it protects
/// never fires, and nothing anywhere reports a problem. This release was spent
/// closing exactly that shape of defect: `net --resource-types` read three key
/// names, none of which any producer wrote, and answered `ok: true` with zero
/// rows on every page for as long as it shipped.
///
/// Referring to the name instead makes that typo a compile error, which is the
/// whole return on the change. The occurrences that merely NAVIGATE to the page
/// gain consistency; the fourteen that compare against it gain a guarantee.
pub const ABOUT_BLANK: &str = "about:blank";
