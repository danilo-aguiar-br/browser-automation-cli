// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ref map and active-frame tracking.

use rustc_hash::FxHashMap;

/// What a `@eN` ref points at, recorded when the snapshot minted it.
///
/// A ref resolves through EITHER `backend_node_id` (accessibility snapshot) OR
/// `selector` (CSS path); the two paths are exclusive, and which one is set
/// decides how the element is found again at interaction time.
#[derive(Debug, Clone)]
pub struct RefEntry {
    /// CDP backend node id, for refs minted from the accessibility tree.
    ///
    /// Survives DOM mutation better than a `nodeId`, but only within the life of
    /// the page: it is meaningless to a later process.
    pub backend_node_id: Option<i64>,
    /// Accessibility role, kept so `view` can print the element as the user saw it.
    pub role: String,
    /// Accessible name, same purpose as `role`.
    pub name: String,
    /// Index among same-role, same-name siblings, when one is needed to disambiguate.
    pub nth: Option<usize>,
    /// CSS selector, for refs minted from a query instead of a snapshot.
    pub selector: Option<String>,
    /// Frame the element lives in. `None` means the main frame.
    pub frame_id: Option<String>,
}

/// Ref id → entry map for the current snapshot.
///
/// Uses [`FxHashMap`]: ref ids are short process-minted tokens (`e1`, `e2`, …),
/// not untrusted external keys (rules_rust_eficiencia_e_performance).
pub struct RefMap {
    map: FxHashMap<String, RefEntry>,
    next_ref: usize,
}

impl Default for RefMap {
    fn default() -> Self {
        Self::new()
    }
}

impl RefMap {
    /// Empty map whose first minted ref will be `e1`.
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            next_ref: 1,
        }
    }

    /// Record a snapshot ref in the main frame.
    pub fn add(
        &mut self,
        ref_id: String,
        backend_node_id: Option<i64>,
        role: &str,
        name: &str,
        nth: Option<usize>,
    ) {
        self.add_with_frame(ref_id, backend_node_id, role, name, nth, None);
    }

    /// Record a snapshot ref, naming the frame it belongs to.
    ///
    /// Frame-aware refs are what let a later `press @eN` reach into an iframe
    /// without the caller re-selecting the frame.
    pub fn add_with_frame(
        &mut self,
        ref_id: String,
        backend_node_id: Option<i64>,
        role: &str,
        name: &str,
        nth: Option<usize>,
        frame_id: Option<&str>,
    ) {
        self.map.insert(
            ref_id,
            RefEntry {
                backend_node_id,
                role: role.to_string(),
                name: name.to_string(),
                nth,
                selector: None,
                frame_id: frame_id.map(|s| s.to_string()),
            },
        );
    }

    /// Record a ref that resolves through a CSS selector instead of a node id.
    pub fn add_selector(
        &mut self,
        ref_id: String,
        selector: String,
        role: &str,
        name: &str,
        nth: Option<usize>,
    ) {
        self.map.insert(
            ref_id,
            RefEntry {
                backend_node_id: None,
                role: role.to_string(),
                name: name.to_string(),
                nth,
                selector: Some(selector),
                frame_id: None,
            },
        );
    }

    /// Look up a ref by its bare id (`e1`, not `@e1`).
    pub fn get(&self, ref_id: &str) -> Option<&RefEntry> {
        self.map.get(ref_id)
    }

    /// Every ref in mint order (`e1`, `e2`, … `e10`), not hash order.
    ///
    /// Sorted NUMERICALLY on the id suffix: lexicographic order would put `e10`
    /// before `e2` and renumber the snapshot the agent reads.
    pub fn entries_sorted(&self) -> Vec<(String, RefEntry)> {
        let mut entries = self
            .map
            .iter()
            .map(|(ref_id, entry)| (ref_id.clone(), entry.clone()))
            .collect::<Vec<_>>();

        entries.sort_by_key(|(ref_id, _)| {
            ref_id
                .strip_prefix('e')
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(usize::MAX)
        });

        entries
    }

    /// Drop a single ref, leaving the counter alone.
    pub fn remove(&mut self, ref_id: &str) {
        self.map.remove(ref_id);
    }

    /// Drop every ref and restart numbering at `e1`.
    ///
    /// Called when the page navigates: keeping refs across a navigation would
    /// hand back ids that point at a document that no longer exists.
    pub fn clear(&mut self) {
        self.map.clear();
        self.next_ref = 1;
    }

    /// The number the next minted ref will use.
    pub fn next_ref_num(&self) -> usize {
        self.next_ref
    }

    /// Resume numbering from `n`, so a second snapshot extends the first
    /// instead of reusing ids that are already on screen.
    pub fn set_next_ref_num(&mut self, n: usize) {
        self.next_ref = n;
    }
}

/// Accept the three spellings of a ref and return the bare id.
///
/// `@e1`, `ref=e1` and `e1` all yield `e1`. Anything else is `None`, which is
/// what lets a caller treat an argument as a CSS selector when it is not a ref.
pub fn parse_ref(input: &str) -> Option<String> {
    let trimmed = input.trim();

    if let Some(stripped) = trimmed.strip_prefix('@') {
        if stripped.starts_with('e') && stripped[1..].chars().all(|c| c.is_ascii_digit()) {
            return Some(stripped.to_string());
        }
    }

    if let Some(stripped) = trimmed.strip_prefix("ref=") {
        if stripped.starts_with('e') && stripped[1..].chars().all(|c| c.is_ascii_digit()) {
            return Some(stripped.to_string());
        }
    }

    if trimmed.starts_with('e')
        && trimmed.len() > 1
        && trimmed[1..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(trimmed.to_string());
    }

    None
}

/// Mirror of session active frame id, refreshed before every command
/// (commands are serialized by the session state lock, so this cannot
/// race). It lets CSS-selector resolution honor `frame <sel>` without
/// threading a parameter through every interaction signature; snapshot refs
/// already carry their frame through the ref map.
///
/// # Interior mutability choice
///
/// - Needs a process-wide `Sync` static with a non-`Copy` payload (`String`) →
///   `std::sync::Mutex` (not `Cell`/`RefCell`/`Atomic*`).
/// - Not `tokio::sync::Mutex`: accessors are sync and never hold the guard
///   across `.await`.
/// - Direct `Mutex::new` (MSRV ≥ 1.63) — no `OnceLock`/`LazyLock` wrapper.
/// - Poison is recovered via [`crate::sync_util::lock_recover`] so a prior panic
///   cannot sticky-fail frame ops.
static ACTIVE_FRAME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub(super) fn active_frame() -> Option<String> {
    crate::sync_util::lock_recover(&ACTIVE_FRAME).clone()
}
