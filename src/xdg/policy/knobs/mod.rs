// SPDX-License-Identifier: MIT OR Apache-2.0
//! Promoted-policy table split into mechanism (`expand`) and data (`table`).

mod expand;
mod table;

pub use table::{
    key, policy_apply_raw, policy_default, policy_list_entries, policy_pairs, policy_set,
    policy_stored, PolicyConfig, POLICY_KEYS,
};
