// SPDX-License-Identifier: MIT OR Apache-2.0
//! The QWERTY layout, and the two timing facts derived from it.
//!
//! Split out because both facts are properties of the KEYBOARD, not of the
//! operator: which finger types a character decides how fast the next one can
//! follow, and which key sits next to it decides which wrong key a hand hits.
//! Neither has a value an operator could sensibly choose differently, which is
//! why this file holds constants and `knobs/table.rs` holds none of them.
//!
//! # Why a layout and not a bigram table
//!
//! The reference implementation named in `docs/STEALTH_PARITY.md` ships a
//! table of hot digraphs. A table is a measurement of one corpus in one
//! language; copying its numbers here would be citing a source this product
//! never read. The layout is the CAUSE behind that table — `th` is fast
//! because `t` and `h` are typed by opposite hands, and `qz` is slow because
//! both are the left little finger moving two rows — so deriving from the
//! layout reproduces the ordering without borrowing numbers.

/// Finger that types `ch` on a QWERTY board, `0` left little to `7` right little.
///
/// `None` for anything not on the main block: a character this file cannot
/// place must not be given a made-up finger, because a wrong placement is a
/// timing claim the layout does not support.
fn finger(ch: char) -> Option<u8> {
    Some(match ch.to_ascii_lowercase() {
        '1' | 'q' | 'a' | 'z' => 0,
        '2' | 'w' | 's' | 'x' => 1,
        '3' | 'e' | 'd' | 'c' => 2,
        '4' | '5' | 'r' | 't' | 'f' | 'g' | 'v' | 'b' => 3,
        '6' | '7' | 'y' | 'u' | 'h' | 'j' | 'n' | 'm' => 4,
        '8' | 'i' | 'k' | ',' => 5,
        '9' | 'o' | 'l' | '.' => 6,
        '0' | '-' | '=' | 'p' | '[' | ']' | ';' | '\'' | '/' => 7,
        _ => return None,
    })
}

/// Alternating hands: the next finger is already travelling while this one presses.
const ALTERNATING_HAND_PERMILLE: u64 = 800;

/// The same key twice: no travel at all, so faster than a normal same-hand gap.
const SAME_KEY_PERMILLE: u64 = 900;

/// Same hand, different finger: the ordinary case, and the unit of comparison.
const SAME_HAND_PERMILLE: u64 = 1000;

/// One finger, two keys: it must lift, travel and land before the next press.
const SAME_FINGER_PERMILLE: u64 = 1450;

/// How the gap between `from` and `to` scales, in parts per thousand.
///
/// `1000` means "leave the sampled gap alone", which is also what an unplaceable
/// character gets: a space, an accented letter or a symbol outside the main
/// block has no finger here, and inventing one would make the timing lie.
#[must_use]
pub fn gap_permille(from: char, to: char) -> u64 {
    let (Some(a), Some(b)) = (finger(from), finger(to)) else {
        return SAME_HAND_PERMILLE;
    };
    if a != b {
        return if (a < 4) == (b < 4) {
            SAME_HAND_PERMILLE
        } else {
            ALTERNATING_HAND_PERMILLE
        };
    }
    if from.eq_ignore_ascii_case(&to) {
        SAME_KEY_PERMILLE
    } else {
        SAME_FINGER_PERMILLE
    }
}

/// The physical rows, used only to find what sits beside a key.
const ROWS: [&str; 4] = ["1234567890-=", "qwertyuiop[]", "asdfghjkl;'", "zxcvbnm,./"];

/// A key one column away from `ch`, or `None` when `ch` is not on the board.
///
/// `left` picks the side, so the caller's single random bit decides it and this
/// function stays pure. A key at the edge of its row returns its only
/// neighbour rather than `None`: an edge key is still mistyped, just always in
/// the same direction.
#[must_use]
pub fn neighbour(ch: char, left: bool) -> Option<char> {
    let lower = ch.to_ascii_lowercase();
    let row = ROWS.iter().find(|r| r.contains(lower))?;
    let cells: Vec<char> = row.chars().collect();
    let at = cells.iter().position(|c| *c == lower)?;
    let pick = if left {
        at.checked_sub(1).unwrap_or(at + 1)
    } else if at + 1 < cells.len() {
        at + 1
    } else {
        at - 1
    };
    let hit = *cells.get(pick)?;
    // The case of the intended character carries over, because a hand that
    // holds shift for `T` holds it for the `R` it hits by mistake.
    Some(if ch.is_ascii_uppercase() {
        hit.to_ascii_uppercase()
    } else {
        hit
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_pair_is_faster_than_the_awkward_one() {
        // The exact comparison `docs/STEALTH_PARITY.md` names as the check.
        assert!(
            gap_permille('t', 'h') < gap_permille('q', 'z'),
            "`th` alternates hands and `qz` repeats the left little finger"
        );
    }

    #[test]
    fn a_repeated_key_is_not_charged_the_travel_it_never_makes() {
        assert!(gap_permille('l', 'l') < gap_permille('q', 'z'));
        assert_eq!(gap_permille('l', 'L'), SAME_KEY_PERMILLE);
    }

    #[test]
    fn an_unplaceable_character_leaves_the_gap_alone() {
        assert_eq!(gap_permille(' ', 'a'), SAME_HAND_PERMILLE);
        assert_eq!(gap_permille('a', 'ç'), SAME_HAND_PERMILLE);
    }

    #[test]
    fn every_neighbour_is_adjacent_on_some_row() {
        for ch in "qwertyuiopasdfghjklzxcvbnm".chars() {
            for left in [true, false] {
                let n = neighbour(ch, left).expect("main block letter has a neighbour");
                assert_ne!(n, ch, "{ch} must not be its own typo");
                let row = ROWS
                    .iter()
                    .find(|r| r.contains(ch))
                    .expect("letter sits on a row");
                assert!(row.contains(n), "{n} must share the row of {ch}");
            }
        }
    }

    #[test]
    fn a_typo_keeps_the_shift_the_writer_was_holding() {
        assert_eq!(neighbour('T', false), Some('Y'));
    }

    #[test]
    fn a_character_off_the_board_has_no_typo() {
        assert_eq!(neighbour(' ', true), None);
        assert_eq!(neighbour('ç', false), None);
    }
}
