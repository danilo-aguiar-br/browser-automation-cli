// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded parallelism and concurrency for the one-shot CLI.
//!
//! # Workload classification (rules_rust_paralelismo)
//!
//! | Class | Paths | Tool |
//! |-------|-------|------|
//! | **I/O-bound** | HTTP scrape/crawl/batch, CDP fan-out, robots | Tokio + `Arc<Semaphore>` / `JoinSet` / [`join_bounded`](crate::concurrency::join_bounded) |
//! | **CPU-bound** | Structural scan (`sg`), multi-file text match | Rayon `par_iter` |
//! | **Mista** | Browser session (CDP I/O + light DOM parse) | Multi-thread Tokio; no Rayon on async workers |
//! | **Subprocess** | Chrome residual | Existing residual kill path (no unbounded fork) |
//!
//! # Module map (Pass 32 SRP-08)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | budget | permits, free RAM, auto formula, semaphore |
//! | join | join_bounded / join_bounded_ordered |
//! | pool | Rayon / walk / browser worker threads |
//! | fs_block | spawn_blocking disk helpers |
//! | cpu | map/filter/count/sort CPU helpers |
//! | matrix | budget_report + command workload matrix |
//!
//! One-shot CLI (BORN→EXECUTE→FINALIZE→DIE). Gate: `Arc<Semaphore>` + `acquire_owned`.

mod budget;
mod cpu;
mod fs_block;
mod join;
mod matrix;
mod pool;

#[cfg(test)]
mod tests;

pub use budget::*;
pub use cpu::*;
pub use fs_block::*;
pub use join::*;
pub use matrix::*;
pub use pool::*;
