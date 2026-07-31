// SPDX-License-Identifier: MIT OR Apache-2.0
//! `OneShotSession` interaction methods.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | pointer | hover, drag, drag_ex |
//! | drag_support | drop point, intercept wait, synthetic fallback |
//! | forms | fill_form, upload |
//! | navigation | back, forward, reload, history |
//! | tabs | page_list, page_new, page_select, page_close |
//!
//! Every method stays an inherent method on `OneShotSession`, so call sites are
//! unchanged; only the defining file moved.

mod drag_support;
mod forms;
mod navigation;
mod pointer;
mod tabs;
