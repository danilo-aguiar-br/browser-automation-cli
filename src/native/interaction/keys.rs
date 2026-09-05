// SPDX-License-Identifier: MIT OR Apache-2.0

pub(super) fn char_to_key_info(ch: char) -> (String, String, i32) {
    match ch {
        '\n' | '\r' => ("Enter".to_string(), "Enter".to_string(), 13),
        '\t' => ("Tab".to_string(), "Tab".to_string(), 9),
        ' ' => (" ".to_string(), "Space".to_string(), 32),
        _ => {
            let key = ch.to_string();
            if ch.is_ascii_alphabetic() {
                // For letters the Windows VK code equals the uppercase ASCII value.
                //
                // WINDOWS, and only Windows. Callers must send this as
                // `windowsVirtualKeyCode` and leave `nativeVirtualKeyCode`
                // unset, because the native code is a DIFFERENT namespace on
                // every platform and we never learn it here.
                //
                // Measured 2026-09-04 on macOS: sending 65 in BOTH fields made
                // Chrome read the native one, where 65 is
                // `kVK_ANSI_KeypadDecimal`, and emit `code: NumpadDecimal`,
                // `keyCode: 110`, `key: Unidentified`. One `type --target`
                // with the single character `a` produced 1 correct keydown plus
                // 374 spurious ones in a 213 ms burst, and five characters hung
                // `Input.dispatchKeyEvent` past chromiumoxide 30 s
                // `REQUEST_TIMEOUT`. Linux is wrong the same way: X11 keycode
                // 65 is `space`.
                let upper = ch.to_ascii_uppercase();
                let code = format!("Key{upper}");
                let key_code = upper as i32;
                (key, code, key_code)
            } else if ch.is_ascii_digit() {
                let code = format!("Digit{ch}");
                let key_code = ch as i32;
                (key, code, key_code)
            } else {
                let (code, key_code) = punctuation_key_info(ch);
                (key, code.to_string(), key_code)
            }
        }
    }
}

/// Return the DOM `KeyboardEvent.code` value and Windows virtual-key code for
/// a punctuation / symbol character assuming a US keyboard layout.
///
/// The Windows virtual-key codes (VK_OEM_*) differ from ASCII values for
/// punctuation.  Using the raw ASCII code would misidentify characters – e.g.
/// '.' (ASCII 46) collides with VK_DELETE (0x2E = 46), causing the period to
/// be swallowed.
pub(super) fn punctuation_key_info(ch: char) -> (&'static str, i32) {
    match ch {
        // VK_OEM_1 (0xBA = 186) — ";:" key on US layout
        ';' | ':' => ("Semicolon", 186),
        // VK_OEM_PLUS (0xBB = 187) — "=+" key
        '=' | '+' => ("Equal", 187),
        // VK_OEM_COMMA (0xBC = 188) — ",<" key
        ',' | '<' => ("Comma", 188),
        // VK_OEM_MINUS (0xBD = 189) — "-_" key
        '-' | '_' => ("Minus", 189),
        // VK_OEM_PERIOD (0xBE = 190) — ".>" key
        '.' | '>' => ("Period", 190),
        // VK_OEM_2 (0xBF = 191) — "/?" key
        '/' | '?' => ("Slash", 191),
        // VK_OEM_3 (0xC0 = 192) — "`~" key
        '`' | '~' => ("Backquote", 192),
        // VK_OEM_4 (0xDB = 219) — "[{" key
        '[' | '{' => ("BracketLeft", 219),
        // VK_OEM_5 (0xDC = 220) — "\\|" key
        '\\' | '|' => ("Backslash", 220),
        // VK_OEM_6 (0xDD = 221) — "]}" key
        ']' | '}' => ("BracketRight", 221),
        // VK_OEM_7 (0xDE = 222) — "'\""" key
        '\'' | '"' => ("Quote", 222),
        _ => ("", 0),
    }
}

/// Return the `text` value that CDP `Input.dispatchKeyEvent` needs on the
/// `keyDown` event so that Chrome performs the default action for the key.
/// For example Enter needs `"\r"` to actually submit a form, and Tab needs
/// `"\t"` to move focus.  Non-printable / navigation keys return `None`.
pub(super) fn key_text(key_name: &str) -> Option<String> {
    match key_name {
        "Enter" => Some("\r".to_string()),
        "Tab" => Some("\t".to_string()),
        " " => Some(" ".to_string()),
        _ => {
            // Single printable characters carry themselves as text.
            if key_name.len() == 1 {
                Some(key_name.to_string())
            } else {
                None
            }
        }
    }
}

/// Split a chord like `Control+a` into its base key and CDP modifier bits.
///
/// # Why this exists
///
/// `named_key_info` matches whole names and has no `+` case, so `Control+a`
/// fell to its catch-all and became a KEY literally named `Control+a`, with
/// `keyCode: 0` and no modifier set. Chrome dispatched an event no keyboard can
/// produce and the page ignored it, silently: the CDP call still succeeded.
///
/// Measured consequence in `content::input`: `type --clear --focus-only` sends
/// `Control+a` then `Backspace`, so with select-all never happening the
/// `Backspace` deleted ONE character and the field kept the rest, while the
/// envelope reported success.
///
/// Bits are the CDP `Input.dispatchKeyEvent` set: Alt 1, Control 2, Meta 4,
/// Shift 8.
///
/// A string whose prefix is not a known modifier name is returned untouched,
/// so `a+b` stays the key `a+b` rather than being silently reinterpreted.
pub(super) fn parse_chord(key: &str) -> (String, i32) {
    let Some((prefix, base)) = key.rsplit_once('+') else {
        return (key.to_string(), 0);
    };
    // A trailing `+` means the chord ends ON the plus key (`Shift++`), which
    // `rsplit_once` hands back as an empty base.
    let base = if base.is_empty() { "+" } else { base };
    let mut bits = 0;
    for part in prefix.split('+').filter(|s| !s.is_empty()) {
        bits |= match part.to_ascii_lowercase().as_str() {
            "alt" | "option" => 1,
            "control" | "ctrl" => 2,
            "meta" | "command" | "cmd" | "super" => 4,
            "shift" => 8,
            _ => return (key.to_string(), 0),
        };
    }
    if bits == 0 {
        return (key.to_string(), 0);
    }
    (base.to_string(), bits)
}

pub(super) fn named_key_info(key: &str) -> (String, String, i32) {
    match key.to_lowercase().as_str() {
        "enter" | "return" => ("Enter".to_string(), "Enter".to_string(), 13),
        "tab" => ("Tab".to_string(), "Tab".to_string(), 9),
        "escape" | "esc" => ("Escape".to_string(), "Escape".to_string(), 27),
        "backspace" => ("Backspace".to_string(), "Backspace".to_string(), 8),
        "delete" => ("Delete".to_string(), "Delete".to_string(), 46),
        "arrowup" | "up" => ("ArrowUp".to_string(), "ArrowUp".to_string(), 38),
        "arrowdown" | "down" => ("ArrowDown".to_string(), "ArrowDown".to_string(), 40),
        "arrowleft" | "left" => ("ArrowLeft".to_string(), "ArrowLeft".to_string(), 37),
        "arrowright" | "right" => ("ArrowRight".to_string(), "ArrowRight".to_string(), 39),
        "home" => ("Home".to_string(), "Home".to_string(), 36),
        "end" => ("End".to_string(), "End".to_string(), 35),
        "pageup" => ("PageUp".to_string(), "PageUp".to_string(), 33),
        "pagedown" => ("PageDown".to_string(), "PageDown".to_string(), 34),
        "space" | " " => (" ".to_string(), "Space".to_string(), 32),
        _ => {
            if key.len() == 1 {
                if let Some(ch) = key.chars().next() {
                    char_to_key_info(ch)
                } else {
                    (key.to_string(), key.to_string(), 0)
                }
            } else {
                (key.to_string(), key.to_string(), 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_chord;

    #[test]
    fn chord_splits_into_base_key_and_modifier_bits() {
        // The defect this pins: `Control+a` reached `named_key_info` whole, fell
        // to its catch-all, and became a key literally named `Control+a` with
        // `keyCode: 0` and no modifier. `type --clear --focus-only` sends this
        // exact string, so select-all never happened and the `Backspace` after
        // it deleted ONE character while the envelope reported success.
        assert_eq!(parse_chord("Control+a"), ("a".to_string(), 2));
        assert_eq!(parse_chord("Meta+a"), ("a".to_string(), 4));
        assert_eq!(parse_chord("Shift+Tab"), ("Tab".to_string(), 8));
        assert_eq!(parse_chord("Alt+Shift+x"), ("x".to_string(), 1 | 8));
    }

    #[test]
    fn spellings_of_the_same_modifier_agree() {
        assert_eq!(parse_chord("ctrl+a"), parse_chord("Control+a"));
        assert_eq!(parse_chord("cmd+a"), parse_chord("Meta+a"));
        assert_eq!(parse_chord("CONTROL+a"), parse_chord("control+a"));
    }

    #[test]
    fn a_non_chord_is_returned_untouched() {
        // The property that keeps this from being a regression of its own: a
        // string containing `+` is not automatically a chord. Reinterpreting
        // `a+b` as a modified `b` would silently change what an existing caller
        // sends, which is the same class of defect being fixed here.
        assert_eq!(parse_chord("Enter"), ("Enter".to_string(), 0));
        assert_eq!(parse_chord("a"), ("a".to_string(), 0));
        assert_eq!(parse_chord("a+b"), ("a+b".to_string(), 0));
        assert_eq!(parse_chord("+"), ("+".to_string(), 0));
    }

    #[test]
    fn a_chord_ending_on_the_plus_key_keeps_that_key() {
        // `rsplit_once` hands back an empty base for `Shift++`, which would
        // otherwise dispatch a key with no name at all.
        assert_eq!(parse_chord("Shift++"), ("+".to_string(), 8));
    }
}
