// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-platform host helpers (PATH lookup, console, sandbox, environment).
//!
//! Product law: Chrome CDP is **host-only** (not WASM). Browser path override is
//! XDG `chrome_path` / CLI launch options — not product env vars.
//!
//! Rules: `docs_rules/rules_rust_multiplataforma_sistemas_operacionais.md`.
//!
//! # Modules
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `host` | `HostEnvironment` probe + summary |
//! | `sandbox` | Snap/Flatpak browser sandbox classification |
//! | `path_util` | `which_bin`, executable probes |
//! | `process_util` | timed capture, BatBadBut-safe binary checks |
//! | `windows_registry` | Windows App Paths discovery |
//! | `console` | UTF-8 / VT console setup |
//! | `detect` | WSL / container / CI / Termux detectors |

mod console;
mod detect;
mod host;
mod path_util;
mod process_util;
mod sandbox;
mod windows_registry;

#[cfg(test)]
mod tests;

pub use console::configure_console;
pub use host::HostEnvironment;
pub use path_util::{
    first_existing_executable, is_executable_file, probe_binary_version, which_bin,
};
pub use process_util::{
    arg_contains_nul, is_spawn_safe_binary, run_capture_with_timeout, wait_child_or_kill,
    ProcessCaptureError,
};
pub use sandbox::{detect_browser_sandbox, warn_if_sandboxed_browser, BrowserSandbox};
pub use windows_registry::registry_app_path;
