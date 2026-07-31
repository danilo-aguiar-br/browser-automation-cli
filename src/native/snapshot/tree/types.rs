// SPDX-License-Identifier: MIT OR Apache-2.0
//! Node, hidden-input and duplicate-name types used while building the tree.

use rustc_hash::FxHashMap;

pub(crate) struct TreeNode {
    pub(crate) role: String,
    pub(crate) name: String,
    pub(crate) level: Option<i64>,
    pub(crate) checked: Option<String>,
    pub(crate) expanded: Option<bool>,
    pub(crate) selected: Option<bool>,
    pub(crate) disabled: Option<bool>,
    pub(crate) required: Option<bool>,
    pub(crate) value_text: Option<String>,
    pub(crate) backend_node_id: Option<i64>,
    pub(crate) children: Vec<usize>,
    pub(crate) parent_idx: Option<usize>,
    pub(crate) has_ref: bool,
    pub(crate) ref_id: Option<String>,
    pub(crate) depth: usize,
    pub(crate) cursor_info: Option<CursorElementInfo>,
    pub(crate) url: Option<String>,
}

impl TreeNode {
    // Create an empty node
    pub(crate) fn empty() -> Self {
        Self {
            role: String::new(),
            name: String::new(),
            level: None,
            checked: None,
            expanded: None,
            selected: None,
            disabled: None,
            required: None,
            value_text: None,
            backend_node_id: None,
            children: Vec::new(),
            parent_idx: None,
            has_ref: false,
            ref_id: None,
            depth: 0,
            cursor_info: None,
            url: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.role = String::new();
        self.name = String::new();
        self.level = None;
        self.checked = None;
        self.expanded = None;
        self.selected = None;
        self.disabled = None;
        self.required = None;
        self.value_text = None;
        self.backend_node_id = None;
        self.children.clear();
        self.parent_idx = None;
        self.has_ref = false;
        self.url = None;
        self.ref_id = None;
        self.depth = 0;
        self.cursor_info = None;
    }
}

/// The type of a hidden form input found inside a cursor-interactive element.
#[derive(Clone, Copy)]
pub(crate) enum HiddenInputKind {
    Radio,
    Checkbox,
}

impl HiddenInputKind {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "radio" => Some(Self::Radio),
            "checkbox" => Some(Self::Checkbox),
            _ => None,
        }
    }

    pub(crate) fn as_role(self) -> &'static str {
        match self {
            Self::Radio => "radio",
            Self::Checkbox => "checkbox",
        }
    }
}

/// Information about a cursor-interactive element (elements with cursor:pointer, onclick, tabindex, etc.)
#[derive(Clone)]
pub(crate) struct CursorElementInfo {
    pub(crate) kind: String, // "clickable", "focusable", "editable"
    pub(crate) hints: Vec<String>,
    pub(crate) text: String, // textContent from the DOM element (fallback when ARIA name is empty)
    pub(crate) hidden_input_kind: Option<HiddenInputKind>,
    pub(crate) hidden_input_checked: Option<String>, // "true", "false", or "mixed" (tristate)
}

pub(crate) struct RoleNameTracker {
    /// Process-minted role:name keys — FxHashMap (not SipHash) for snapshot build.
    pub(crate) counts: FxHashMap<String, usize>,
    pub(crate) entries: Vec<(usize, String)>,
}

impl RoleNameTracker {
    pub(crate) fn new() -> Self {
        Self {
            counts: FxHashMap::default(),
            entries: Vec::new(),
        }
    }

    pub(crate) fn track(&mut self, role: &str, name: &str, node_idx: usize) -> usize {
        let key = format!("{role}:{name}");
        let count = self.counts.entry(key.clone()).or_insert(0);
        let nth = *count;
        *count += 1;
        self.entries.push((node_idx, key));
        nth
    }

    pub(crate) fn get_duplicates(&self) -> FxHashMap<String, usize> {
        self.counts
            .iter()
            .filter(|(_, &count)| count > 1)
            .map(|(key, &count)| (key.clone(), count))
            .collect()
    }
}
