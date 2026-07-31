// SPDX-License-Identifier: MIT OR Apache-2.0
//! Promoted operation-policy knobs (GAP-048): XDG keys backed by named constants.
//!
//! # Classification law
//!
//! A constant in [`crate::constants`] is either a **protocol invariant** (its
//! value cannot change without breaking CDP, an argv contract, or an on-disk
//! format) or an **operation policy** (a timeout, size ceiling, count,
//! dimension, or path). Only operation policy is promoted here.
//!
//! # Precedence
//!
//! CLI flag → XDG key (`config set <key> <value>`) → named constant default.
//! The constant remains the single source of truth for the default: promoting a
//! knob never changes its effective value when the key is unset.
//!
//! # Storage
//!
//! Every promoted knob is stored as `Option<u64>` and validated `> 0` on
//! `config set`. Typed accessors cast down and fall back to the constant when
//! the stored value does not fit the target width.
//!
//! # Snapshot
//!
//! This is a one-shot process: the XDG file is read at most once per process
//! and cached in a [`std::sync::OnceLock`], so hot loops (event pump slices, poll
//! intervals) never re-read disk.

mod access;
mod knobs;
mod validate;

#[cfg(test)]
mod tests;

pub use access::{
    is_policy_key, policy_i32, policy_millis, policy_secs, policy_u32, policy_u64, policy_usize,
};
pub use knobs::{
    key, policy_apply_raw, policy_default, policy_list_entries, policy_pairs, policy_set,
    policy_stored, PolicyConfig, POLICY_KEYS,
};
