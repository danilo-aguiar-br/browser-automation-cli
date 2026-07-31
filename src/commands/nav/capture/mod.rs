// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot artifact capture handlers.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Artifact |
//! |--------|----------|
//! | eval | JavaScript evaluation result |
//! | screenshot | `grab` image |
//! | pdf | `print-pdf` document |
//! | monitor | Baseline hash change report |
//! | extract | DOM text / attribute |
//! | extract_llm | LLM-backed structured extract |
//!
//! Handlers are re-exported flat so existing paths keep working.

mod eval;
mod extract;
mod extract_llm;
mod monitor;
mod pdf;
mod screenshot;

pub(crate) use eval::*;
pub(crate) use extract::*;
pub(crate) use monitor::*;
pub(crate) use pdf::*;
pub(crate) use screenshot::*;
