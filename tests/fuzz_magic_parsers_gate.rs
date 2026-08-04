// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic fuzzing of the magic-byte parsers (GAP-IMG-096).
//!
//! # Why this replaces the cargo-fuzz recipe
//!
//! `docs/TESTING.md` has carried a `cargo fuzz init && cargo fuzz add` recipe
//! since auditoria-04. No `fuzz/` directory ever existed, so the gap was closed
//! on paper and open in fact. Three properties of that recipe kept it from being
//! run: it needs a nightly toolchain, it needs libFuzzer from LLVM — a C++
//! dependency in a crate whose whole premise is rust-native — and it is a
//! separate binary that no gate invokes.
//!
//! This gate trades exhaustive coverage for the property that actually matters
//! and can run everywhere: the parsers are handed arbitrary bytes and must
//! either classify them or return a typed error. They must never panic, never
//! index out of bounds, and never loop forever.
//!
//! # Why the generator is a hand-rolled PRNG
//!
//! Determinism is the point. A seeded xorshift means a failure is reproducible
//! from the seed printed in the assertion, on any machine, with no new
//! dependency and no `Math::random`-style irreproducibility. `proptest` and
//! `arbitrary` would both do this better, and both would be a new dev-dependency
//! for a property expressible in twenty lines.
//!
//! # What the corpus is shaped like
//!
//! Pure noise almost never reaches past the first branch of a magic matcher, so
//! it proves little. Each case therefore starts from a real container prefix and
//! then corrupts it, which is what drives execution into the length fields and
//! box walkers where an overflow would actually live.

use browser_automation_cli::{audio_local, image_local, video_local};

/// Reproducible xorshift64\*. Not cryptographic; it only has to be stable.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}

/// Real container prefixes, so corruption starts from a plausible header.
///
/// A parser that rejects on byte 0 is not being tested; these get past that.
const SEEDS: &[&[u8]] = &[
    // PNG
    b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR",
    // JPEG with an APP1 marker, where EXIF/XMP parsing begins
    b"\xff\xd8\xff\xe1\x00\x10Exif\x00\x00",
    // JPEG with APP13, where the Photoshop 8BIM/IPTC walker begins
    b"\xff\xd8\xff\xed\x00\x1cPhotoshop 3.0\x008BIM",
    // GIF89a, animation control block follows
    b"GIF89a\x10\x00\x10\x00\x80\x00\x00",
    // RIFF/WEBP
    b"RIFF\x24\x00\x00\x00WEBP VP8 ",
    // ISOBMFF ftyp — MP4, HEIF and AVIF all enter through this box
    b"\x00\x00\x00\x18ftypisom\x00\x00\x02\x00",
    b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00",
    b"\x00\x00\x00\x18ftypavif\x00\x00\x00\x00",
    // Matroska / WebM
    b"\x1a\x45\xdf\xa3\x01\x00\x00\x00",
    // OGG
    b"OggS\x00\x02\x00\x00\x00\x00\x00\x00",
    // FLAC
    b"fLaC\x00\x00\x00\x22",
    // MP3 with an ID3v2 header, whose size field is synchsafe
    b"ID3\x04\x00\x00\x00\x00\x00\x00",
    // ADTS AAC
    b"\xff\xf1\x50\x80\x00\x1f\xfc",
    // WAV
    b"RIFF\x24\x00\x00\x00WAVEfmt ",
    // AIFF
    b"FORM\x00\x00\x00\x12AIFF",
];

/// One corrupted sample: a real prefix, mutated, then extended with noise.
fn sample(rng: &mut Rng, cap: usize) -> Vec<u8> {
    let seed = SEEDS[rng.below(SEEDS.len())];
    let mut buf = seed.to_vec();

    // Truncation is its own bug class: a length field promising more than the
    // buffer holds is exactly where a naive parser slices out of bounds.
    if rng.below(4) == 0 && buf.len() > 1 {
        let keep = rng.below(buf.len());
        buf.truncate(keep);
    }

    // Flip a few bytes in place, which is what drives a walker down a branch its
    // author never considered.
    let flips = rng.below(6);
    for _ in 0..flips {
        if buf.is_empty() {
            break;
        }
        let at = rng.below(buf.len());
        buf[at] = rng.byte();
    }

    // Extend with noise up to the cap.
    let extra = rng.below(cap.saturating_sub(buf.len()).max(1));
    for _ in 0..extra {
        buf.push(rng.byte());
    }
    buf
}

/// Every magic parser survives arbitrary input without panicking.
///
/// The assertion is the absence of a panic: `catch_unwind` would report one as a
/// test failure anyway, so the loop itself is the gate. What is asserted
/// explicitly is that the call returns — a hang would show up as a timeout, and
/// the iteration count is sized to finish in well under a second.
#[test]
fn magic_parsers_never_panic_on_arbitrary_bytes() {
    const ITERATIONS: usize = 4_000;
    const MAX_LEN: usize = 512;
    // Fixed seed: a failure here is reproducible by rerunning the test.
    let mut rng = Rng(0x5EED_1234_ABCD_0001);

    for i in 0..ITERATIONS {
        let bytes = sample(&mut rng, MAX_LEN);

        // Each parser returns Result; either arm is acceptable. What is not
        // acceptable is unwinding, which the harness turns into a failure.
        let img = image_local::detect_format(&bytes);
        let vid = video_local::detect_container(&bytes);
        let aud = audio_local::detect_container(&bytes);

        // Touch the results so nothing is optimised away, and give a failing
        // iteration a reproducible identity in the message.
        assert!(
            img.is_ok() || img.is_err(),
            "iteration {i}: image parser returned neither arm"
        );
        assert!(
            vid.is_ok() || vid.is_err(),
            "iteration {i}: video parser returned neither arm"
        );
        assert!(
            aud.is_ok() || aud.is_err(),
            "iteration {i}: audio parser returned neither arm"
        );
    }
}

/// Degenerate inputs the corpus generator would rarely produce on its own.
///
/// Empty, one byte, and a buffer that is all zeros are the cases a length-driven
/// parser is most likely to mishandle, and the least likely to be reached by
/// mutation from a valid header.
#[test]
fn magic_parsers_handle_degenerate_inputs() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0x00],
        vec![0xff],
        vec![0x00; 64],
        vec![0xff; 64],
        // ISOBMFF box claiming a size larger than the buffer.
        b"\xff\xff\xff\xffftypisom".to_vec(),
        // ISOBMFF box claiming size 0, which means "to end of file".
        b"\x00\x00\x00\x00ftypisom".to_vec(),
        // ID3v2 header with a maximal synchsafe size and no payload.
        b"ID3\x04\x00\x00\x7f\x7f\x7f\x7f".to_vec(),
    ];

    for (i, bytes) in cases.iter().enumerate() {
        let _ = image_local::detect_format(bytes);
        let _ = video_local::detect_container(bytes);
        let _ = audio_local::detect_container(bytes);
        // Reaching here means no parser unwound on case `i`.
        assert!(bytes.len() < 1024, "case {i}: fixture unexpectedly large");
    }
}
