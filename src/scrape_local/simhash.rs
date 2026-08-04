// SPDX-License-Identifier: MIT OR Apache-2.0
//! SimHash content fingerprints for near-duplicate page collapsing.
//!
//! Charikar SimHash over word shingles, implemented in-tree: the algorithm is a
//! few dozen lines and does not justify a dependency.
//!
//! # How it works
//!
//! 1. The text is lowercased and split on non-alphanumeric boundaries.
//! 2. Consecutive words are grouped into shingles of
//!    [`crate::constants::SCRAPE_SIMHASH_SHINGLE_WORDS`] words, so word order
//!    matters and a reshuffled page is not mistaken for the same page.
//! 3. Each shingle is hashed to 64 bits; every set bit votes `+1` and every
//!    clear bit votes `-1` in a 64-wide accumulator.
//! 4. The sign of each accumulator lane becomes the corresponding output bit.
//!
//! Two documents that share most shingles therefore agree on most bits, so
//! near-duplicate detection reduces to a Hamming distance threshold.
//!
//! # Workload
//!
//! **CPU-bound but linear** in text length with no allocation per shingle
//! beyond a small rolling window; safe to call inline on already-fetched bodies.

use std::hash::{Hash, Hasher};

/// A 64-bit SimHash content fingerprint.
///
/// `Copy` and cheap to compare: near-duplicate testing is one XOR plus a
/// population count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimHash(u64);

impl SimHash {
    /// Compute the fingerprint of `text`.
    ///
    /// Text with fewer words than one shingle still hashes (the whole text
    /// becomes a single shingle); empty or punctuation-only text yields `0`,
    /// which [`SimHash::is_empty`] reports so callers can decline to collapse
    /// pages whose extracted content was blank.
    pub fn of(text: &str) -> Self {
        let words: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            return Self(0);
        }
        let width = crate::constants::SCRAPE_SIMHASH_SHINGLE_WORDS.max(1);
        let mut lanes = [0i32; u64::BITS as usize];
        for start in 0..words.len() {
            let end = (start + width).min(words.len());
            accumulate(&mut lanes, shingle_hash(&words[start..end]));
            if end == words.len() {
                // The tail shingle is shorter than `width`; further starts would
                // only re-hash suffixes of it, biasing the tail of the document.
                break;
            }
        }
        let mut bits = 0u64;
        for (i, lane) in lanes.iter().enumerate() {
            if *lane > 0 {
                bits |= 1u64 << i;
            }
        }
        Self(bits)
    }

    /// Raw 64-bit fingerprint, for envelope emission or persistence.
    pub fn bits(self) -> u64 {
        self.0
    }

    /// Lowercase hex rendering of the fingerprint (stable, 16 chars).
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }

    /// True when no shingle contributed — blank or punctuation-only text.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Number of differing bits between two fingerprints (0..=64).
    pub fn distance(self, other: Self) -> u32 {
        (self.0 ^ other.0).count_ones()
    }

    /// True when the two fingerprints are within `max_distance` bits.
    pub fn is_near_duplicate(self, other: Self, max_distance: u32) -> bool {
        self.distance(other) <= max_distance
    }
}

/// Fold one shingle hash into the 64 signed lanes.
fn accumulate(lanes: &mut [i32; u64::BITS as usize], hash: u64) {
    for (i, lane) in lanes.iter_mut().enumerate() {
        if hash & (1u64 << i) != 0 {
            *lane = lane.saturating_add(1);
        } else {
            *lane = lane.saturating_sub(1);
        }
    }
}

/// Hash a shingle (slice of consecutive words), case-insensitively.
fn shingle_hash(words: &[&str]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    for w in words {
        // Lowercase per word so `Rust` and `rust` shingle identically without
        // allocating a lowercased copy of the whole document.
        for c in w.chars().flat_map(char::to_lowercase) {
            c.hash(&mut hasher);
        }
        0u8.hash(&mut hasher); // separator: "ab c" must not collide with "a bc"
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_has_distance_zero() {
        let a = SimHash::of("the quick brown fox jumps over the lazy dog");
        let b = SimHash::of("the quick brown fox jumps over the lazy dog");
        assert_eq!(a.distance(b), 0);
        assert!(a.is_near_duplicate(b, 0));
    }

    #[test]
    fn case_and_punctuation_do_not_change_fingerprint() {
        let a = SimHash::of("The Quick, Brown Fox!");
        let b = SimHash::of("the quick brown fox");
        assert_eq!(a.distance(b), 0);
    }

    #[test]
    fn near_duplicate_text_stays_within_band() {
        let base = "Rust is a systems programming language focused on safety \
                    speed and concurrency without a garbage collector";
        let edited = "Rust is a systems programming language focused on safety \
                      speed and concurrency without any garbage collector";
        let d = SimHash::of(base).distance(SimHash::of(edited));
        assert!(d > 0, "a real edit must move at least one bit");
        assert!(d <= 16, "one-word edit should stay near, got {d}");
    }

    #[test]
    fn unrelated_text_is_far_apart() {
        let a = SimHash::of(
            "quarterly financial results revenue growth margins guidance investors earnings",
        );
        let b = SimHash::of(
            "baking sourdough bread requires patience starter hydration flour oven steam",
        );
        let d = a.distance(b);
        assert!(d > 8, "unrelated documents should exceed the band, got {d}");
    }

    #[test]
    fn word_order_matters() {
        let a = SimHash::of("alpha beta gamma delta epsilon zeta");
        let b = SimHash::of("zeta epsilon delta gamma beta alpha");
        assert!(
            a.distance(b) > 0,
            "shingling must make reordering observable"
        );
    }

    #[test]
    fn blank_text_is_empty_fingerprint() {
        assert!(SimHash::of("").is_empty());
        assert!(SimHash::of("   ... !!! ").is_empty());
        assert!(!SimHash::of("content").is_empty());
    }

    #[test]
    fn short_text_below_shingle_width_still_hashes() {
        let a = SimHash::of("hi");
        assert!(!a.is_empty());
        assert_eq!(a.distance(SimHash::of("hi")), 0);
    }

    #[test]
    fn hex_is_stable_and_sized() {
        let h = SimHash::of("stable output").to_hex();
        assert_eq!(h.len(), 16);
        assert_eq!(h, SimHash::of("stable output").to_hex());
    }
}
