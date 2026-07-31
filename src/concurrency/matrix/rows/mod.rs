// SPDX-License-Identifier: MIT OR Apache-2.0
//! Static command-by-command workload rows (PAR-73 honesty data).
//!
//! # Module map (GAP-051 SRP split)
//!
//! Rows are grouped by command family, one module per family. The table order
//! emitted by [`command_rows`] is the concatenation order of the modules below
//! and is identical to the previous single-file table.
//!
//! | Module | Family |
//! |--------|--------|
//! | meta | doctor, commands, schema, version, locale |
//! | interaction | goto … upload |
//! | navigation | back, forward, reload |
//! | artifacts | eval, grab, print-pdf, monitor |
//! | scripting | run, exec |
//! | inspection | extract … dialog |
//! | scraping | scrape … qr |
//! | local_tools | find-paths … config |
//! | profiling | emulate … heap |
//! | platform | extension … residual |
//! | nested | dotted subcommand keys (PAR-76) |

use std::sync::OnceLock;

mod artifacts;
mod inspection;
mod interaction;
mod local_tools;
mod meta;
mod navigation;
mod nested;
mod platform;
mod profiling;
mod scraping;
mod scripting;

/// (name, class, gate?, reason?)
pub(super) type CmdRow = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
);

/// Full command posture table. Nested multi-item subcommands use dotted keys (PAR-76).
pub(super) fn command_rows() -> &'static [CmdRow] {
    static TABLE: OnceLock<Vec<CmdRow>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let groups: [&[CmdRow]; 11] = [
            meta::ROWS,
            interaction::ROWS,
            navigation::ROWS,
            artifacts::ROWS,
            scripting::ROWS,
            inspection::ROWS,
            scraping::ROWS,
            local_tools::ROWS,
            profiling::ROWS,
            platform::ROWS,
            nested::ROWS,
        ];
        groups.concat()
    })
}
