// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome discovery + launch option args for chromiumoxide one-shot.
//!
//! FORBIDDEN: dual spawn via Child/Command for Chrome production path.
//! FORBIDDEN: BrowserFetcher embedded no MVP (system Chrome only).
//! Launch ownership: `oxide::launch_with_oxide` → `Browser::launch`.
//!
//! # Workload (PAR-92 / PAR-101)
//!
//! **Subprocess + I/O:** the profile path is allocated in `build_chrome_args`
//! without `std::fs` on the async worker. Materialization uses
//! `materialize_profile_dir`, which wraps
//! [`crate::concurrency::create_dir_all_blocking`], and covers BOTH the owned
//! temp profile and an operator's explicit `--profile`. Tests may call
//! `materialize_user_data_dir_sync`.
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `options` | `LaunchOptions` |
//! | `args` | `ChromeArgs` / `build_chrome_args` / sandbox helpers |
//! | `discover` | `find_chrome` + OS path candidates |
//! | `tooling` | puppeteer/playwright cache paths |

mod args;
mod discover;
mod options;
mod process;
mod spawn;
mod tooling;

#[cfg(test)]
mod tests;

pub use args::merge_proxy_bypass;
pub use discover::find_chrome;
pub use options::LaunchOptions;
pub use process::ChromeProcess;
pub use spawn::{launch_self_spawned, ChromeLaunch};

pub(crate) use args::{build_chrome_args, launch_args, materialize_profile_dir};
