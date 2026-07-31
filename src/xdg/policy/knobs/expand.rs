// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `policy_knobs!` expansion: mechanism, with no rows of its own.
//!
//! Separated from the table so a new knob is a one-line data edit and the
//! expansion logic stays readable on one screen.

// The macro expands at its INVOCATION site, so the items it names must be
// in scope in `table.rs`, not here. Macro hygiene does not carry imports.

macro_rules! policy_knobs {
    ($( $konst:ident => $key:ident , $desc:literal ; )*) => {
        /// Promoted policy knobs parsed from the XDG config file.
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct PolicyConfig {
            $(
                #[doc = $desc]
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $key: Option<u64>,
            )*
        }

        /// Compile-time key name constants (call sites never spell raw strings).
        pub mod key {
            $(
                #[doc = $desc]
                pub const $konst: &str = stringify!($key);
            )*
        }

        /// Canonical list of promoted policy key names.
        pub const POLICY_KEYS: &[&str] = &[ $( stringify!($key), )* ];

        /// Named constant default for a policy key.
        pub fn policy_default(key: &str) -> Option<u64> {
            match key {
                $( stringify!($key) => Some(crate::constants::$konst as u64), )*
                _ => None,
            }
        }

        /// Stored override for a policy key (`None` when the key is unknown).
        pub fn policy_stored(cfg: &PolicyConfig, key: &str) -> Option<Option<u64>> {
            match key {
                $( stringify!($key) => Some(cfg.$key), )*
                _ => None,
            }
        }

        /// Permissive loader arm for the loose TOML reader (invalid values drop).
        pub fn policy_apply_raw(cfg: &mut PolicyConfig, key: &str, raw: &str) -> bool {
            match key {
                $(
                    stringify!($key) => {
                        cfg.$key = raw.parse::<u64>().ok().filter(|&n| n > 0);
                        true
                    }
                )*
                _ => false,
            }
        }

        /// Strict `config set` arm; `None` when the key is not a policy key.
        pub fn policy_set(cfg: &mut PolicyConfig, key: &str, raw: &str) -> Option<Result<(), CliError>> {
            let parsed = match parse_policy_value(key, raw) {
                Ok(n) => n,
                Err(e) => return Some(Err(e)),
            };
            match key {
                $( stringify!($key) => { cfg.$key = Some(parsed); Some(Ok(())) } )*
                _ => None,
            }
        }

        /// Non-default overrides, in canonical order, for the TOML writer.
        pub fn policy_pairs(cfg: &PolicyConfig) -> Vec<(&'static str, u64)> {
            let mut out = Vec::new();
            $( if let Some(v) = cfg.$key { out.push((stringify!($key), v)); } )*
            out
        }

        /// `config list-keys` entries for every promoted policy key.
        pub fn policy_list_entries() -> Vec<serde_json::Value> {
            vec![ $(
                serde_json::json!({
                    "key": stringify!($key),
                    "default": crate::constants::$konst as u64,
                    "description": $desc,
                }),
            )* ]
        }
    };
}

// `macro_rules!` is textually scoped; this makes it reachable from `table`.
pub(crate) use policy_knobs;
