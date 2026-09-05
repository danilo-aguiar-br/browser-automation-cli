// SPDX-License-Identifier: MIT OR Apache-2.0
//! Heap snapshot parsing ceilings and node-op result caps.

/// Offline heap snapshot file size ceiling (bytes).
pub const HEAP_SNAPSHOT_MAX_BYTES: u64 = 512 * 1024 * 1024;
/// Heap node-op default max retainers returned.
pub const HEAP_DEFAULT_MAX_RETAINERS: usize = 200;
/// Heap node-op default max edges returned.
pub const HEAP_DEFAULT_MAX_EDGES: usize = 200;
/// Heap paths enumeration max paths.
pub const HEAP_DEFAULT_MAX_PATHS: usize = 32;
/// Heap paths max depth.
pub const HEAP_DEFAULT_MAX_PATH_DEPTH: usize = 8;
/// Heap class_nodes list cap.
pub const HEAP_DEFAULT_MAX_CLASS_NODES: usize = 500;
/// Dominator visited-state ceiling (anti-pathological graphs).
pub const HEAP_DOMINATOR_MAX_STATES: usize = 50_000;

/// Preview length of a duplicated string, in CHARACTERS.
///
/// Characters, not bytes. The strings come from the page's own JavaScript
/// heap, so they carry arbitrary UTF-8: emoji, CJK, accented Latin. Cutting
/// by byte index splits a code point whenever the boundary lands mid-sequence,
/// and `&str` indexing panics there rather than truncating. Under the release
/// profile's `panic = "abort"` that is a SIGABRT, not a recoverable error.
pub const HEAP_DUP_STRING_PREVIEW_CHARS: usize = 120;
/// Duplicate-string groups emitted by `heap dup-strings`.
pub const HEAP_DUP_LIST_CAP: usize = 50;

/// Heap snapshot outer poll max iterations.
pub const DEFAULT_HEAP_OUTER_ITERS: u32 = 200;
/// Heap snapshot inner drain iterations after finished.
pub const DEFAULT_HEAP_INNER_ITERS: u32 = 10;
/// Heap snapshot final drain iterations.
pub const DEFAULT_HEAP_FINAL_ITERS: u32 = 20;
