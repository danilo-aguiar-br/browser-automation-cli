// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP protocol message types (split by domain for SRP).

mod accessibility;
mod core;
mod dom;
mod input;
mod network;
mod page;
mod runtime;
mod screenshot;
mod target;

pub use accessibility::*;
pub use core::*;
pub use dom::*;
pub use input::*;
pub use network::*;
pub use page::*;
pub use runtime::*;
pub use screenshot::*;
pub use target::*;

pub mod generated {
    //! CDP wire types generated from `cdp-protocol/*.json` by `build.rs`.
    //!
    //! Every item here mirrors a Chrome DevTools Protocol type 1:1, so the
    //! authoritative documentation is the protocol itself:
    //! <https://chromedevtools.github.io/devtools-protocol/>.
    //!
    //! GAP-046: `missing_docs` is allowed for the generated items only. Writing
    //! per-item rustdoc for a machine-emitted wire surface would restate the
    //! field name and drift from the protocol on every regeneration; the fix
    //! belongs to the generator, not to hand-edited copies of its output. The
    //! crate-level `missing_docs` warning stays in force everywhere else.
    #![allow(missing_docs)]

    include!(concat!(env!("OUT_DIR"), "/cdp_generated.rs"));
}
