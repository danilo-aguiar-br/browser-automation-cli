// SPDX-License-Identifier: MIT OR Apache-2.0
//! Per-snapshot `nth` assignment for repeated role+name pairs.

use super::DurableLocator;

/// Assign `nth` across a snapshot so repeated role+name pairs stay addressable.
///
/// Input is `(role, name)` in snapshot order; output is one locator per entry.
pub fn assign_locators(pairs: &[(String, String)]) -> Vec<DurableLocator> {
    let mut totals: std::collections::HashMap<(&str, &str), usize> =
        std::collections::HashMap::new();
    for (role, name) in pairs {
        *totals.entry((role.as_str(), name.as_str())).or_insert(0) += 1;
    }
    let mut seen: std::collections::HashMap<(&str, &str), usize> = std::collections::HashMap::new();
    pairs
        .iter()
        .map(|(role, name)| {
            let key = (role.as_str(), name.as_str());
            let counter = seen.entry(key).or_insert(0);
            *counter += 1;
            let unique = totals.get(&key).copied().unwrap_or(1) == 1;
            DurableLocator::new(role, name, if unique { None } else { Some(*counter) })
        })
        .collect()
}
