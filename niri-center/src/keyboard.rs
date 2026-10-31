//! Keyboards as the Keybinds page pictures them: the rows of keys on a
//! layout, by the keysym names niri binds use, and the caps' labels.

use crate::bindings::key_id;

/// The key a combo ends on: `T` in `Mod+Shift+T`.
pub fn last_key(combo: &str) -> &str {
    combo.rsplit('+').next().unwrap_or("")
}

/// Whether two key names are the same key: niri reads them in any case.
pub fn same_key(first: &str, second: &str) -> bool {
    key_id(first) == key_id(second)
}

/// One key of a [`Picture`]: the keysym name niri binds use for it, and
/// what its cap shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub token: String,
    pub label: String,
}

/// A keyboard as the Keybinds page pictures it, rows of keys from the top.
///
/// Keys hold the keysym names niri binds use on that layout: niri matches
/// binds against the active layout's keysyms, so on `de` the key labelled
/// Z sits where US has Y. Tokens are matched case-insensitively against
/// the key each combo ends on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Picture {
    pub rows: Vec<Vec<Key>>,
}

impl Default for Picture {
    /// US, xkb's layout when none is set.
    fn default() -> Self {
        Self::drawn(&US_ROWS)
    }
}

/// What niri builds its keymap from: the config's `xkb` block.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Xkb {
    pub rules: String,
    pub model: String,
    /// Layout codes in order, and each one's variant at the same place.
    pub layouts: Vec<String>,
    pub variants: Vec<String>,
    pub options: String,
    /// A keymap file niri reads instead of the names above.
    pub file: Option<String>,
}

impl Xkb {
    /// The variant the config gives layout `code`; empty when none.
    pub fn variant_of(&self, code: &str) -> &str {
        self.layouts
            .iter()
            .position(|layout| layout == code)
            .and_then(|index| self.variants.get(index))
            .map_or("", String::as_str)
    }
}

impl Picture {
    /// Layout `code` as xkb lays out its keys, with the config's rules,
    /// model, options and variant for it, as niri compiles them. Without
    /// xkb (the browser demo), or for a layout xkb cannot compile, the
    /// drawn picture of `code`, when there is one.
    pub fn of(xkb: &Xkb, code: &str) -> Option<Self> {
        if let Some(picture) = compiled::picture(xkb, code) {
            return Some(picture);
        }
        match base_layout(code) {
            "us" => Some(Self::drawn(&US_ROWS)),
            "de" => Some(Self::drawn(&DE_ROWS)),
            _ => None,
        }
    }

    fn drawn(rows: &[&[&str]]) -> Self {
        Self {
            rows: rows
                .iter()
                .map(|row| row.iter().map(|token| fixed(token)).collect())
                .collect(),
        }
    }
}

/// A key's character as its cap shows it: upper case, unless that is more
/// than one letter (`ß` stays `ß`, not `SS`).
pub fn capital(character: &str) -> String {
    let upper = character.to_uppercase();
    if upper.chars().count() == 1 {
        upper
    } else {
        character.to_string()
    }
}

/// A keysym name as niri's own config spells it (`L`, `Comma`), where a
/// drawn picture has the key; xkb's name (`l`, `comma`) otherwise.
#[cfg(not(target_arch = "wasm32"))]
fn spelled(name: String) -> String {
    US_ROWS
        .iter()
        .chain(DE_ROWS.iter())
        .flat_map(|row| row.iter())
        .find(|drawn| key_id(drawn) == key_id(&name))
        .map_or(name, |drawn| (*drawn).to_string())
}

/// `us` for `us(intl)`: the layout without its variant.
fn base_layout(code: &str) -> &str {
    code.split('(').next().unwrap_or("").trim()
}

/// A key every layout has, labelled as [`key_label`] says.
fn fixed(token: &str) -> Key {
    Key {
        token: token.to_string(),
        label: key_label(token).to_string(),
    }
}

/// The browser demo has no libxkbcommon: no layout compiles there.
#[cfg(target_arch = "wasm32")]
mod compiled {
    use super::{Picture, Xkb};

    pub fn picture(_xkb: &Xkb, _code: &str) -> Option<Picture> {
        None
    }
}

/// Keymaps compiled with libxkbcommon, the library niri reads its config's
/// `xkb` block with.
#[cfg(not(target_arch = "wasm32"))]
mod compiled {
    use xkbcommon::xkb;

    use super::{Key, MOD_ROW, NAV_ROW, Picture, Xkb, base_layout, capital, fixed, spelled};

    /// A key's place in a row: an xkb keycode (evdev's plus 8) whose
    /// symbol the keymap gives, or a key that is the same on every layout.
    #[derive(Clone, Copy)]
    enum Place {
        Code(u32),
        Same(FixedKey),
    }

    /// A key that is the same on every layout, by its xkb name.
    #[derive(Clone, Copy)]
    enum FixedKey {
        BackSpace,
        Return,
        Tab,
        CapsLock,
        Shift,
    }

    impl FixedKey {
        const fn token(self) -> &'static str {
            match self {
                Self::BackSpace => "BackSpace",
                Self::Return => "Return",
                Self::Tab => "Tab",
                Self::CapsLock => "Caps_Lock",
                Self::Shift => "Shift",
            }
        }
    }

    /// The character keys' rows: ISO, with the key beside left Shift and
    /// Enter two rows high, unless `ansi`.
    fn places(ansi: bool) -> Vec<Vec<Place>> {
        let codes = |range: std::ops::RangeInclusive<u32>| range.map(Place::Code);
        let number_row = std::iter::once(Place::Code(49))
            .chain(codes(10..=21))
            .chain([Place::Same(FixedKey::BackSpace)])
            .collect();
        let (top_end, home_end) = if ansi {
            (Place::Code(51), Place::Same(FixedKey::Return))
        } else {
            (Place::Same(FixedKey::Return), Place::Code(51))
        };
        let top = std::iter::once(Place::Same(FixedKey::Tab))
            .chain(codes(24..=35))
            .chain([top_end])
            .collect();
        let home = std::iter::once(Place::Same(FixedKey::CapsLock))
            .chain(codes(38..=48))
            .chain([home_end])
            .collect();
        let beside_shift = (!ansi).then_some(Place::Code(94));
        let bottom = std::iter::once(Place::Same(FixedKey::Shift))
            .chain(beside_shift)
            .chain(codes(52..=61))
            .chain([Place::Same(FixedKey::Shift)])
            .collect();
        vec![number_row, top, home, bottom]
    }

    pub fn picture(xkb: &Xkb, code: &str) -> Option<Picture> {
        let variant = xkb.variant_of(code);
        // the names go to C: one with a NUL in it is not a layout
        if [&xkb.rules, &xkb.model, &xkb.options]
            .iter()
            .any(|name| name.contains('\0'))
            || code.contains('\0')
            || variant.contains('\0')
        {
            return None;
        }
        let mut context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        // a layout it does not know is answered with None, not on stderr
        context.set_log_level(xkb::LogLevel::Critical);
        let options = (!xkb.options.is_empty()).then(|| xkb.options.clone());
        let keymap = xkb::Keymap::new_from_names(
            &context,
            xkb.rules.as_str(),
            xkb.model.as_str(),
            code,
            variant,
            options,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )?;
        let ansi = base_layout(code) == "us";
        let mut rows: Vec<Vec<Key>> = places(ansi)
            .into_iter()
            .map(|row| row.into_iter().map(|place| key(&keymap, place)).collect())
            .collect();
        rows.push(MOD_ROW.iter().map(|token| fixed(token)).collect());
        rows.push(NAV_ROW.iter().map(|token| fixed(token)).collect());
        Some(Picture { rows })
    }

    /// The key at `place`: its symbol without Shift on the first layout,
    /// labelled with the character it types, upper case.
    fn key(keymap: &xkb::Keymap, place: Place) -> Key {
        let code = match place {
            Place::Same(fixed_key) => return fixed(fixed_key.token()),
            Place::Code(code) => code,
        };
        let Some(&symbol) = keymap
            .key_get_syms_by_level(xkb::Keycode::new(code), 0, 0)
            .first()
        else {
            return Key {
                token: String::new(),
                label: String::new(),
            };
        };
        let token = spelled(xkb::keysym_get_name(symbol));
        let typed = xkb::keysym_to_utf8(symbol);
        let label = if typed.chars().count() == 1 && !typed.chars().all(char::is_control) {
            capital(&typed)
        } else {
            dead_key_label(&token).unwrap_or(&token).to_string()
        };
        Key { token, label }
    }

    /// What a dead key adds to the next letter, as its cap shows it.
    fn dead_key_label(token: &str) -> Option<&'static str> {
        Some(match token.strip_prefix("dead_")? {
            "grave" => "`",
            "acute" => "´",
            "circumflex" => "^",
            "tilde" => "~",
            "diaeresis" => "¨",
            "cedilla" => "¸",
            "caron" => "ˇ",
            "breve" => "˘",
            "abovering" => "°",
            "abovedot" => "˙",
            "doubleacute" => "˝",
            "ogonek" => "˛",
            "macron" => "¯",
            _ => return None,
        })
    }
}

/// Modifier and navigation rows shared by every layout.
const MOD_ROW: &[&str] = &["Ctrl", "Super", "Alt", "Space", "Alt", "Super", "Ctrl"];
const NAV_ROW: &[&str] = &[
    "Escape",
    "Print",
    "Delete",
    "Home",
    "End",
    "Page_Up",
    "Page_Down",
    "Left",
    "Down",
    "Up",
    "Right",
];

/// US ANSI.
const US_ROWS: [&[&str]; 6] = [
    &[
        "Grave",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "0",
        "Minus",
        "Equal",
        "BackSpace",
    ],
    &[
        "Tab",
        "Q",
        "W",
        "E",
        "R",
        "T",
        "Y",
        "U",
        "I",
        "O",
        "P",
        "BracketLeft",
        "BracketRight",
        "Backslash",
    ],
    &[
        "Caps_Lock",
        "A",
        "S",
        "D",
        "F",
        "G",
        "H",
        "J",
        "K",
        "L",
        "Semicolon",
        "Apostrophe",
        "Return",
    ],
    &[
        "Shift", "Z", "X", "C", "V", "B", "N", "M", "Comma", "Period", "Slash", "Shift",
    ],
    MOD_ROW,
    NAV_ROW,
];

/// German ISO (QWERTZ).
const DE_ROWS: [&[&str]; 6] = [
    &[
        "dead_circumflex",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "0",
        "ssharp",
        "dead_acute",
        "BackSpace",
    ],
    &[
        "Tab",
        "Q",
        "W",
        "E",
        "R",
        "T",
        "Z",
        "U",
        "I",
        "O",
        "P",
        "udiaeresis",
        "plus",
        "Return",
    ],
    &[
        "Caps_Lock",
        "A",
        "S",
        "D",
        "F",
        "G",
        "H",
        "J",
        "K",
        "L",
        "odiaeresis",
        "adiaeresis",
        "numbersign",
    ],
    &[
        "Shift", "less", "Y", "X", "C", "V", "B", "N", "M", "Comma", "Period", "Minus", "Shift",
    ],
    MOD_ROW,
    NAV_ROW,
];

/// Short cap label for a keysym; punctuation shows its glyph.
pub fn key_label(token: &str) -> &str {
    match token {
        "Grave" => "`",
        "Minus" => "-",
        "Equal" => "=",
        "BackSpace" => "Backspace",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Backslash" => "\\",
        "Caps_Lock" => "Caps",
        "Semicolon" => ";",
        "Apostrophe" => "'",
        "Return" => "Enter",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Escape" => "Esc",
        "Delete" => "Del",
        "Page_Up" => "PgUp",
        "Page_Down" => "PgDn",
        "Left" => "←",
        "Down" => "↓",
        "Up" => "↑",
        "Right" => "→",
        "dead_circumflex" => "^",
        "ssharp" => "ß",
        "dead_acute" => "´",
        "udiaeresis" => "Ü",
        "odiaeresis" => "Ö",
        "adiaeresis" => "Ä",
        "plus" => "+",
        "numbersign" => "#",
        "less" => "<",
        other => other,
    }
}

/// Relative width of special keys (others are 1 unit).
pub fn key_width(token: &str) -> f32 {
    match token {
        // xkb names the backslash key in lower case
        "BackSpace" | "Shift" | "Caps_Lock" | "Backslash" | "backslash" => 1.8,
        "Tab" | "Return" => 1.5,
        "Ctrl" | "Super" | "Alt" => 1.3,
        "Space" => 5.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xkb's own `de` and `us`, compiled from xkeyboard-config, and the
    /// pictures drawn by hand from the keyboards: two sources that must
    /// agree on every key and cap.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn xkb_lays_out_us_and_de_as_drawn() {
        let xkb = Xkb::default();
        for (code, drawn) in [("us", &US_ROWS), ("de", &DE_ROWS)] {
            let compiled = compiled::picture(&xkb, code).expect("xkb compiles the layout");
            let drawn = Picture::drawn(drawn);
            assert_eq!(compiled.rows.len(), drawn.rows.len(), "{code}");
            for (compiled_row, drawn_row) in compiled.rows.iter().zip(&drawn.rows) {
                assert_eq!(compiled_row, drawn_row, "{code}");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_variant_changes_the_keys_it_changes() {
        let xkb = Xkb {
            layouts: vec!["us".into(), "de".into()],
            variants: vec![String::new(), "nodeadkeys".into()],
            ..Xkb::default()
        };
        let first = |picture: Picture| picture.rows[0][0].token.clone();
        assert_eq!(first(compiled::picture(&xkb, "de").unwrap()), "asciicircum");
        assert_eq!(
            first(compiled::picture(&Xkb::default(), "de").unwrap()),
            "dead_circumflex"
        );
        // AZERTY: A where QWERTY has Q
        assert_eq!(compiled::picture(&xkb, "fr").unwrap().rows[1][1].token, "A");
    }

    #[test]
    fn a_layout_nobody_knows_has_no_picture() {
        assert_eq!(Picture::of(&Xkb::default(), "no-such-layout"), None);
        assert!(Picture::of(&Xkb::default(), "us(intl)").is_some());
    }

    #[test]
    fn variants_line_up_with_layouts_by_place() {
        let xkb = Xkb {
            layouts: vec!["de".into(), "us".into()],
            variants: vec![String::new(), "dvorak".into()],
            ..Xkb::default()
        };
        assert_eq!(xkb.variant_of("us"), "dvorak");
        assert_eq!(xkb.variant_of("de"), "");
        assert_eq!(xkb.variant_of("fr"), "");
    }

    #[test]
    fn a_cap_keeps_a_letter_with_no_single_capital() {
        assert_eq!(capital("ü"), "Ü");
        assert_eq!(capital("ß"), "ß");
    }
}
