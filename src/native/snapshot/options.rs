// SPDX-License-Identifier: MIT OR Apache-2.0
//! Snapshot options and interactive-role tables.

pub(super) const INTERACTIVE_ROLES: &[&str] = &[
    "button",
    "link",
    "textbox",
    "checkbox",
    "radio",
    "combobox",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "searchbox",
    "slider",
    "spinbutton",
    "switch",
    "tab",
    "treeitem",
    "Iframe",
];

pub(super) const CONTENT_ROLES: &[&str] = &[
    "heading",
    "cell",
    "gridcell",
    "columnheader",
    "rowheader",
    "listitem",
    "article",
    "region",
    "main",
    "navigation",
];

pub(super) const INVISIBLE_CHARS: &[char] = &[
    '\u{FEFF}', // BOM / Zero Width No-Break Space
    '\u{200B}', // Zero Width Space
    '\u{200C}', // Zero Width Non-Joiner
    '\u{200D}', // Zero Width Joiner
    '\u{2060}', // Word Joiner
    '\u{00A0}', // Non-Breaking Space (&nbsp;)
];

#[derive(Default)]
/// How much of the accessibility tree a snapshot should return.
///
/// Every field here trades completeness for tokens. The default answers "what
/// can I act on", not "what does the page contain", because an agent pays for
/// the whole tree it did not ask for.
pub struct SnapshotOptions {
    /// Root the walk at this element instead of the document.
    pub selector: Option<String>,
    /// Keep only actionable roles (button, link, textbox, …).
    pub interactive: bool,
    /// Drop decoration from each line, keeping role, name and ref.
    pub compact: bool,
    /// Stop descending after this many levels. `None` walks the whole tree.
    pub depth: Option<usize>,
    /// Include link targets, which are large but often what the caller is after.
    pub urls: bool,
}
