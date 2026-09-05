// SPDX-License-Identifier: MIT OR Apache-2.0
//! The half of the JSON contract that touches a FILE.
//!
//! # Where the seam is
//!
//! [`super`] holds rules about the JSON grammar itself — BOM stripping, strict
//! RFC 8259 parsing, the compact wire form, per-line NDJSON ceilings — and none
//! of them need a path. Everything here needs one, and everything here can fail
//! for reasons the grammar knows nothing about: a file too large to allocate,
//! bytes that are not UTF-8, a parent directory that does not exist, a rename
//! that races.
//!
//! That split is not cosmetic. The two failure vocabularies are different, and
//! keeping them in one file is what let the STRICT reader get adopted on a CSV
//! path that had always been lossy: the caller reached for a byte ceiling and
//! took a decoder along with it without noticing. The ceiling is single-sourced
//! in [`stat_within_limit`] precisely so the decoder stays a separate, spelled
//! out choice.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::{CliError, ErrorKind};

use super::from_str;

/// Reject a file whose size exceeds `max_bytes`, returning the measured length.
///
/// Shared by [`read_text_file_limited`] and [`read_text_file_limited_lossy`] so
/// the two decoders can never drift apart on the ceiling they enforce. The
/// decoder is the only thing that differs between them; the limit is not.
fn stat_within_limit(path: &Path, max_bytes: u64) -> Result<u64, CliError> {
    let meta = fs::metadata(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("stat {}: {e}", path.display())))?;
    let len = meta.len();
    if len > max_bytes {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "file {} is too large ({} bytes > {max_bytes})",
                path.display(),
                len
            ),
            crate::i18n::suggestion_key("split_input_or_raise_limit", None),
        ));
    }
    Ok(len)
}

/// Reserve `len` bytes up front, surfacing allocation failure instead of aborting.
fn reserve_for<T>(buf: &mut Vec<T>, len: u64, path: &Path) -> Result<(), CliError> {
    buf.try_reserve_exact(len as usize).map_err(|e| {
        CliError::new(
            ErrorKind::Software,
            format!("reserve {} bytes for {}: {e}", len, path.display()),
        )
    })
}

/// Own a BOM-free string for downstream `from_str` / line iterators.
fn strip_bom(raw: String) -> String {
    if raw.starts_with('\u{FEFF}') {
        raw.trim_start_matches('\u{FEFF}').to_string()
    } else {
        raw
    }
}

/// Classify a `read_to_string` failure.
///
/// # Why invalid UTF-8 is `Data` and not `Io`
///
/// `read_to_string` reports "stream did not contain valid UTF-8" as
/// [`io::ErrorKind::InvalidData`], and forwarding that as [`ErrorKind::Io`]
/// exits `74` (`EX_IOERR`). Sysexits reserves `74` for a fault in the I/O
/// *system*, which an agent is right to read as transient and retry. No retry
/// converts latin-1 into UTF-8, so that loop never converges. Bytes the user
/// supplied are input data — `65` (`EX_DATAERR`) — and the suggestion names the
/// only fix that works: re-encode the file.
fn classify_read_err(path: &Path, e: &io::Error) -> CliError {
    if e.kind() == io::ErrorKind::InvalidData {
        return CliError::with_suggestion(
            ErrorKind::Data,
            format!("file {} is not valid UTF-8: {e}", path.display()),
            crate::i18n::suggestion_key("file_not_utf8", None),
        );
    }
    CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display()))
}

/// Read a **strict** UTF-8 text file with an explicit byte ceiling.
///
/// Returns the file body **without** a leading BOM (stripped after read).
///
/// # Which reader to pick
///
/// Strict is correct where the format itself mandates UTF-8 — JSON does, by
/// RFC 8259 — so a non-UTF-8 byte is a malformed document and rejecting it is
/// the honest answer. It is the WRONG reader for formats that carry no encoding
/// guarantee, notably HTML and CSV, where windows-1252 and latin-1 are ordinary
/// on the wire and from Excel. Those callers want
/// [`read_text_file_limited_lossy`], which shares this ceiling and differs only
/// in the decoder.
pub fn read_text_file_limited(path: &Path, max_bytes: u64) -> Result<String, CliError> {
    let len = stat_within_limit(path, max_bytes)?;
    let mut raw = String::new();
    raw.try_reserve_exact(len as usize).map_err(|e| {
        CliError::new(
            ErrorKind::Software,
            format!("reserve {} bytes for {}: {e}", len, path.display()),
        )
    })?;
    let file = File::open(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("open {}: {e}", path.display())))?;
    let mut reader = io::BufReader::new(file);
    use std::io::Read;
    reader
        .read_to_string(&mut raw)
        .map_err(|e| classify_read_err(path, &e))?;
    Ok(strip_bom(raw))
}

/// Read a text file **lossily** under the same ceiling as [`read_text_file_limited`].
///
/// Invalid bytes become `U+FFFD` instead of failing the command. Returns the
/// body **without** a leading BOM.
///
/// # Why this exists as a sibling and not as a flag
///
/// A ceiling and a decoder are separate decisions, and collapsing them into one
/// helper is how the strict reader got adopted on a CSV path that had always
/// been lossy: the ceiling was the reason to reach for it, and the decoder came
/// along unnoticed. Two named functions make the decoder a choice the caller
/// has to spell out, while `stat_within_limit` keeps the limit single-sourced.
///
/// The `from_utf8` attempt is not an optimisation detour: it consumes the
/// buffer when the document is already valid UTF-8, which is the common case,
/// whereas `from_utf8_lossy(..).into_owned()` copies even then because
/// `Cow::Borrowed::into_owned` allocates.
pub fn read_text_file_limited_lossy(path: &Path, max_bytes: u64) -> Result<String, CliError> {
    let len = stat_within_limit(path, max_bytes)?;
    let mut bytes: Vec<u8> = Vec::new();
    reserve_for(&mut bytes, len, path)?;
    let file = File::open(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("open {}: {e}", path.display())))?;
    let mut reader = io::BufReader::new(file);
    use std::io::Read;
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display())))?;
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    Ok(strip_bom(text))
}

/// Read + parse a typed JSON file (BOM + size limited).
pub fn read_json_file<T: DeserializeOwned>(path: &Path, max_bytes: u64) -> Result<T, CliError> {
    let raw = read_text_file_limited(path, max_bytes)?;
    from_str(&raw).map_err(|e| super::map_parse_err(&format!("parse {}", path.display()), &e))
}

/// Read + parse a dynamic JSON [`Value`] from a file.
pub fn read_json_value_file(path: &Path, max_bytes: u64) -> Result<Value, CliError> {
    read_json_file(path, max_bytes)
}

/// Atomic JSON write: temp file in same directory → `BufWriter` → flush → rename.
///
/// `pretty = true` only for human-edited artifacts (state dumps, MITM capture review).
/// Machine pipelines should pass `pretty = false`.
pub fn write_json_file_atomic<T: Serialize>(
    path: &Path,
    value: &T,
    pretty: bool,
) -> Result<(), CliError> {
    // GAP-026: every JSON artifact lands inside an allowed root.
    crate::fs_roots::ensure_write_allowed(path)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                CliError::new(
                    ErrorKind::Io,
                    format!("create parent {}: {e}", parent.display()),
                )
            })?;
        }
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("json")
    ));
    // Prefer a unique tmp when extension rewrite collides.
    let tmp = if tmp == path {
        path.with_extension("json.tmp")
    } else {
        tmp
    };
    {
        let file = File::create(&tmp).map_err(|e| {
            CliError::new(ErrorKind::Io, format!("create temp {}: {e}", tmp.display()))
        })?;
        let mut writer = BufWriter::new(file);
        if pretty {
            serde_json::to_writer_pretty(&mut writer, value)
                .map_err(|e| CliError::new(ErrorKind::Software, format!("json encode: {e}")))?;
        } else {
            serde_json::to_writer(&mut writer, value)
                .map_err(|e| CliError::new(ErrorKind::Software, format!("json encode: {e}")))?;
        }
        writer
            .write_all(b"\n")
            .map_err(|e| CliError::new(ErrorKind::Io, format!("json trailing newline: {e}")))?;
        writer
            .flush()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("json flush: {e}")))?;
        writer
            .into_inner()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("json into_inner: {e}")))?
            .sync_all()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("json fsync: {e}")))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        CliError::new(
            ErrorKind::Io,
            format!("rename {} → {}: {e}", tmp.display(), path.display()),
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_JSON_FILE_BYTES, UTF8_BOM};
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn read_file_limited_and_bom() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(UTF8_BOM).unwrap();
        f.write_all(br#"{"a":1}"#).unwrap();
        f.flush().unwrap();
        let v: Value = read_json_file(f.path(), MAX_JSON_FILE_BYTES).unwrap();
        assert_eq!(v["a"], 1);
    }

    /// Invalid UTF-8 is DATA (`65`), never I/O (`74`).
    ///
    /// The distinction is not cosmetic: `74` (`EX_IOERR`) reads as a transient
    /// system fault and invites a retry, and no retry re-encodes the file, so
    /// the agent loops forever on an input only the user can fix.
    #[test]
    fn strict_reader_classifies_invalid_utf8_as_data_not_io() {
        let mut f = NamedTempFile::new().unwrap();
        // `José` in windows-1252: `0xE9` is not valid UTF-8 in any position.
        f.write_all(b"Jos\xe9").unwrap();
        f.flush().unwrap();
        let err = read_text_file_limited(f.path(), MAX_JSON_FILE_BYTES).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Data);
        assert_eq!(err.kind().exit_code(), 65);
    }

    /// The lossy sibling accepts the same bytes the strict one rejects.
    #[test]
    fn lossy_reader_accepts_what_strict_rejects() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"Jos\xe9").unwrap();
        f.flush().unwrap();
        assert!(read_text_file_limited(f.path(), MAX_JSON_FILE_BYTES).is_err());
        let text = read_text_file_limited_lossy(f.path(), MAX_JSON_FILE_BYTES).unwrap();
        assert_eq!(text, "Jos\u{FFFD}");
    }

    /// Both readers enforce the SAME ceiling, and it is still `Data`.
    ///
    /// A size ceiling that only one of the pair honours is how a lossy caller
    /// would end up unbounded, which is the defect the shared helper exists to
    /// prevent — so the limit is asserted on both, not just on the strict one.
    #[test]
    fn both_readers_share_the_byte_ceiling() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"0123456789").unwrap();
        f.flush().unwrap();
        let strict = read_text_file_limited(f.path(), 4).unwrap_err();
        let lossy = read_text_file_limited_lossy(f.path(), 4).unwrap_err();
        assert_eq!(strict.kind(), ErrorKind::Data);
        assert_eq!(lossy.kind(), ErrorKind::Data);
    }

    /// The lossy reader strips a BOM exactly like the strict one.
    #[test]
    fn lossy_reader_strips_bom() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(UTF8_BOM).unwrap();
        f.write_all(b"a,b").unwrap();
        f.flush().unwrap();
        let text = read_text_file_limited_lossy(f.path(), MAX_JSON_FILE_BYTES).unwrap();
        assert_eq!(text, "a,b");
    }
}
