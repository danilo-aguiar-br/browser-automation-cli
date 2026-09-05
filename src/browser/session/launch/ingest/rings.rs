// SPDX-License-Identifier: MIT OR Apache-2.0
//! What happens at the CEILING of a capture buffer, for every buffer at once.
//!
//! # Why the eviction rule is not next to the events
//!
//! [`super`] decodes CDP: it reads a method name and decides which buffer a
//! payload belongs in. Nothing there has an opinion about what to do when a
//! buffer is full, and the moment it does, the opinion gets written once per
//! arm. That is exactly how the screencast ring came to keep the OLDEST frames
//! with `if len < CAP { push }` while the console and network rings were being
//! taught to drop the oldest and COUNT the loss — three buffers, two opposite
//! policies, in one file, with every gate green.
//!
//! One generic function is the fix, and its own module is what stops the next
//! arm from growing a private copy. `cap` arrives as an ARGUMENT rather than
//! being read from XDG inside the eviction, so the policy is a pure function a
//! test can pin without a configured environment.
//!
//! # The buffer this deliberately does NOT serve
//!
//! `heap_chunks` is one JSON value delivered in slices, so dropping its oldest
//! rows produces a syntactically invalid document rather than a shorter valid
//! one. It gets a byte budget and a refusal instead, in [`super`]. Same
//! symptom, opposite remedy — which is the distinction this file exists to keep
//! legible.

use serde_json::Value;

use super::super::super::OneShotSession;

impl OneShotSession {
    /// Push onto a capture buffer, dropping the OLDEST rows past the ring cap.
    ///
    /// `console_log` and `network_log` had no ceiling at all: a page running
    /// `setInterval(console.log, 0)`, or one that keeps fetching, grew them
    /// until the process died. The cap is the XDG knob
    /// `event_tracker_max_entries`, whose own description already calls it the
    /// in-memory console and network ring — the constant and the key both
    /// existed while the one-shot session ignored them.
    ///
    /// The drop is COUNTED and surfaced, never silent. A buffer that quietly
    /// forgets its oldest rows answers with a subset and calls it the whole
    /// set, which is the exact shape of the defect this release closes.
    pub(super) fn push_capped(buffer: &mut Vec<Value>, dropped: &mut u64, entry: Value) {
        buffer.push(entry);
        Self::cap_ring(buffer, dropped, Self::ring_cap());
    }

    /// The configured ring size, or `0` for unbounded.
    fn ring_cap() -> usize {
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::EVENT_TRACKER_MAX_ENTRIES)
    }

    /// Hold `buffer` at `cap` by evicting from the FRONT, counting what left.
    ///
    /// Generic because the console and network rings hold `Value` while the
    /// trace ring holds pre-serialised `String`, and the eviction policy is the
    /// same for all three — writing it once per type is how the three would
    /// drift, which is the defect class this release is closing elsewhere.
    ///
    /// Split out from [`Self::push_capped`] for a second reason: the cap used
    /// to be read from XDG inside the eviction itself, so the policy could not
    /// be exercised without a configured environment. Taking `cap` as an
    /// argument makes it a pure function, and pure is what a test can pin.
    /// `cap == 0` means unbounded, matching the XDG convention.
    pub(super) fn cap_ring<T>(buffer: &mut Vec<T>, dropped: &mut u64, cap: usize) {
        if cap == 0 || buffer.len() <= cap {
            return;
        }
        let excess = buffer.len() - cap;
        buffer.drain(0..excess);
        *dropped = dropped.saturating_add(excess as u64);
    }

    /// Ring cap for the string buffers, mirroring [`Self::push_capped`].
    ///
    /// Split from the `Value` version for a reason that is not type plumbing:
    /// `Tracing.dataCollected` stores pre-serialised NDJSON lines, and an
    /// NDJSON line survives being separated from its neighbours, so dropping
    /// the oldest leaves a shorter but still VALID document. That is not true
    /// of `heap_chunks`, which is one JSON value delivered in slices — which is
    /// why the heap gets a byte budget and a refusal instead of a share in
    /// this function. Same symptom, opposite remedy.
    pub(super) fn push_capped_str(buffer: &mut Vec<String>, dropped: &mut u64, entry: String) {
        buffer.push(entry);
        Self::cap_ring(buffer, dropped, Self::ring_cap());
    }
}
