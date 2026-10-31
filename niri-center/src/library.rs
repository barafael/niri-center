//! The rule library: ready-made window rules for things people often ask
//! niri for, most of them for one app, each naming where it comes from.
//! Marked rules are imported in one change, into a file of their own that
//! the config includes, or into the config itself.
//!
//! Every rule is checked against `niri validate` by the test suite.

use crate::words::Words;
use std::collections::HashSet;

use crate::store::{RuleMeaning, Store, StoreError, WindowRuleRow};
use crate::system::SystemData;

/// Where imported rules go when not into the config itself.
pub const FILE: &str = "rules-library.kdl";

/// An app the library has rules for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownApp {
    NiriCenter,
    Firefox,
    Wezterm,
    Steam,
    Keepassxc,
    Secrets,
    Bitwarden,
    OnePassword,
    Telegram,
    Gimp,
    Pavucontrol,
    Pwvucontrol,
    GnomeCalculator,
    Kcalc,
    NetworkConnections,
    Blueman,
    QtSettings,
    Mpv,
    Obs,
    Fcitx,
}

impl KnownApp {
    pub const ALL: [Self; 20] = [
        Self::NiriCenter,
        Self::Firefox,
        Self::Wezterm,
        Self::Steam,
        Self::Keepassxc,
        Self::Secrets,
        Self::Bitwarden,
        Self::OnePassword,
        Self::Telegram,
        Self::Gimp,
        Self::Pavucontrol,
        Self::Pwvucontrol,
        Self::GnomeCalculator,
        Self::Kcalc,
        Self::NetworkConnections,
        Self::Blueman,
        Self::QtSettings,
        Self::Mpv,
        Self::Obs,
        Self::Fcitx,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::NiriCenter => "niri-center",
            Self::Firefox => "Firefox",
            Self::Wezterm => "WezTerm",
            Self::Steam => "Steam",
            Self::Keepassxc => "KeePassXC",
            Self::Secrets => "Secrets",
            Self::Bitwarden => "Bitwarden",
            Self::OnePassword => "1Password",
            Self::Telegram => "Telegram",
            Self::Gimp => "GIMP",
            Self::Pavucontrol => "Volume control (pavucontrol)",
            Self::Pwvucontrol => "Volume control (pwvucontrol)",
            Self::GnomeCalculator => "Calculator",
            Self::Kcalc => "KCalc",
            Self::NetworkConnections => "Network connections",
            Self::Blueman => "Bluetooth (Blueman)",
            Self::QtSettings => "Qt settings (qt5ct, qt6ct)",
            Self::Mpv => "mpv",
            Self::Obs => "OBS Studio",
            Self::Fcitx => "Fcitx5",
        }
    }

    /// The app IDs niri sees on its windows, across the ways it is
    /// packaged; `niri-center` also covers `niri-center.displays`.
    pub const fn ids(self) -> &'static [&'static str] {
        match self {
            Self::NiriCenter => &["niri-center"],
            Self::Firefox => &["firefox", "org.mozilla.firefox"],
            Self::Wezterm => &["org.wezfurlong.wezterm"],
            Self::Steam => &["steam"],
            Self::Keepassxc => &["org.keepassxc.KeePassXC"],
            Self::Secrets => &["org.gnome.World.Secrets"],
            Self::Bitwarden => &["Bitwarden"],
            Self::OnePassword => &["1Password"],
            Self::Telegram => &["org.telegram.desktop"],
            Self::Gimp => &["gimp"],
            Self::Pavucontrol => &["org.pulseaudio.pavucontrol", "pavucontrol"],
            Self::Pwvucontrol => &["com.saivert.pwvucontrol"],
            Self::GnomeCalculator => &["org.gnome.Calculator", "gnome-calculator"],
            Self::Kcalc => &["org.kde.kcalc"],
            Self::NetworkConnections => &["nm-connection-editor"],
            Self::Blueman => &["blueman-manager"],
            Self::QtSettings => &["qt5ct", "qt6ct"],
            Self::Mpv => &["mpv"],
            Self::Obs => &["com.obsproject.Studio"],
            Self::Fcitx => &["fcitx"],
        }
    }

    /// Desktop file names it installs under when they differ from its ids.
    const fn desktop(self) -> &'static [&'static str] {
        match self {
            Self::Steam => &["com.valvesoftware.Steam"],
            Self::Bitwarden => &["com.bitwarden.desktop"],
            Self::Gimp => &["org.gimp.GIMP"],
            Self::Mpv => &["io.mpv.Mpv"],
            Self::Fcitx => &["org.fcitx.Fcitx5", "fcitx5"],
            Self::NiriCenter
            | Self::Firefox
            | Self::Wezterm
            | Self::Keepassxc
            | Self::Secrets
            | Self::OnePassword
            | Self::Telegram
            | Self::Pavucontrol
            | Self::Pwvucontrol
            | Self::GnomeCalculator
            | Self::Kcalc
            | Self::NetworkConnections
            | Self::Blueman
            | Self::QtSettings
            | Self::Obs => &[],
        }
    }

    /// Whether a window with this app ID is this app's, ignoring case.
    pub fn owns(self, app_id: &str) -> bool {
        self.ids().iter().any(|id| is_app(app_id, id))
    }

    /// Whether it is installed here, by its desktop files.
    pub fn installed(self, system: &SystemData) -> bool {
        self.ids()
            .iter()
            .chain(self.desktop())
            .any(|id| system.has_app(id))
    }
}

/// Whether `app_id` is `id`, or one of its windows named `id.something`.
fn is_app(app_id: &str, id: &str) -> bool {
    app_id.len() >= id.len()
        && app_id.is_char_boundary(id.len())
        && app_id[..id.len()].eq_ignore_ascii_case(id)
        && matches!(app_id.as_bytes().get(id.len()), None | Some(b'.'))
}

/// Where a library rule comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    NiriCenter,
    NiriDefaultConfig,
    NiriWiki,
    SharedConfigs,
    /// niri's issue tracker, by issue number.
    NiriIssues(&'static [u32]),
}

impl std::fmt::Display for Origin {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NiriCenter => out.write_str("niri-center"),
            Self::NiriDefaultConfig => out.write_str("niri's default config"),
            Self::NiriWiki => out.write_str("niri wiki"),
            Self::SharedConfigs => out.write_str("shared niri configs"),
            Self::NiriIssues([only]) => write!(out, "niri issue {only}"),
            Self::NiriIssues(issues) => {
                let numbers: Vec<String> = issues.iter().map(u32::to_string).collect();
                match numbers.split_last() {
                    Some((last, [])) => write!(out, "niri issue {last}"),
                    Some((last, rest)) => write!(out, "niri issues {} and {last}", rest.join(", ")),
                    None => out.write_str("niri's issues"),
                }
            }
        }
    }
}

/// One ready-made rule.
pub struct LibraryRule {
    /// The app it is for; `None`: windows of any app.
    pub app: Option<KnownApp>,
    pub name: Words,
    pub why: Words,
    pub kdl: Words,
    /// Where it comes from.
    pub source: Origin,
    /// Windows it is for, as (app ID, title): the tests check that the
    /// rule reaches each, and that each of its app's IDs is among them.
    pub examples: &'static [(Words, Words)],
}

/// Shared by the small apps that open, get used and close again.
const SMALL_WINDOW: &str =
    "A window you open, use and close again; tiled, it would take a whole column.";

pub const LIBRARY: [LibraryRule; 30] = [
    // ---- every window ----
    LibraryRule {
        app: None,
        name: Words("Rounded corners on every window"),
        why: Words(
            "The focus ring and border follow the corners. Apps that draw their own shadows may look clipped.",
        ),
        kdl: Words(
            r##"window-rule {
    geometry-corner-radius 12
    clip-to-geometry true
}"##,
        ),
        source: Origin::NiriDefaultConfig,
        examples: &[],
    },
    LibraryRule {
        app: None,
        name: Words("Blur behind see-through windows"),
        why: Words(
            "Shows only where a window is transparent, such as a terminal with a translucent background.",
        ),
        kdl: Words(
            r##"window-rule {
    background-effect {
        blur true
    }
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[],
    },
    LibraryRule {
        app: None,
        name: Words("Fade windows that do not have focus"),
        why: Words("Makes the active window stand out without a border."),
        kdl: Words(
            r##"window-rule {
    match is-active=false
    opacity 0.9
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[],
    },
    LibraryRule {
        app: None,
        name: Words("Glow around windows that ask for attention"),
        why: Words(
            "In niri's urgent red, also with the focus ring and border off. Such a window stays fully opaque, even where another rule fades it.",
        ),
        kdl: Words(
            r##"window-rule {
    match is-urgent=true
    opacity 1.0
    shadow {
        on
        softness 40
        spread 8
        draw-behind-window true
        color "#9b0000cc"
    }
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[],
    },
    LibraryRule {
        app: None,
        name: Words("Red ring around a window being screencast"),
        why: Words(
            "When you share a single window, you can see which one, even with the focus ring off.",
        ),
        kdl: Words(
            r##"window-rule {
    match is-window-cast-target=true
    focus-ring {
        on
        active-color "#f38ba8"
        inactive-color "#7d0d2d"
    }
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[],
    },
    LibraryRule {
        app: None,
        name: Words("Picture-in-Picture videos float"),
        why: Words("Firefox and Chromium name the window Picture-in-Picture."),
        kdl: Words(
            r##"window-rule {
    match title="^Picture-in-Picture$"
    open-floating true
}"##,
        ),
        source: Origin::NiriDefaultConfig,
        examples: &[
            (Words("firefox"), Words("Picture-in-Picture")),
            (Words("org.mozilla.firefox"), Words("Picture-in-Picture")),
        ],
    },
    LibraryRule {
        app: None,
        name: Words("File choosers float"),
        why: Words(
            "niri floats a dialog its app claims. Some apps open the file chooser without saying whose it is, and then it tiles.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^xdg-desktop-portal"#
    open-floating true
}"##,
        ),
        source: Origin::NiriIssues(&[2831, 3895]),
        examples: &[
            (Words("xdg-desktop-portal-gtk"), Words("Open File")),
            (Words("xdg-desktop-portal-gnome"), Words("Save File")),
        ],
    },
    LibraryRule {
        app: None,
        name: Words("SSH passphrase prompts float"),
        why: Words("The prompt ssh-askpass opens when a key needs its passphrase."),
        kdl: Words(
            r##"window-rule {
    match title="^OpenSSH Authentication Passphrase request$"
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[(
            Words(""),
            Words("OpenSSH Authentication Passphrase request"),
        )],
    },
    // ---- apps ----
    LibraryRule {
        app: Some(KnownApp::NiriCenter),
        name: Words("Pages in their own window open floating"),
        why: Words(
            "A page opened on its own (Open in its own window, or --pane) floats; the main window keeps tiling.",
        ),
        kdl: Words(crate::store::SUGGESTED_WINDOW_RULES),
        source: Origin::NiriCenter,
        examples: &[(Words("niri-center.displays"), Words("Displays"))],
    },
    LibraryRule {
        app: Some(KnownApp::Firefox),
        name: Words("Private windows stay out of screen recordings"),
        why: Words("They show as a black box to anyone watching a screen share."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"firefox$"# title="Private Browsing"
    block-out-from "screencast"
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[
            (Words("firefox"), Words("Mozilla Firefox Private Browsing")),
            (
                Words("org.mozilla.firefox"),
                Words("Mozilla Firefox Private Browsing"),
            ),
        ],
    },
    LibraryRule {
        app: Some(KnownApp::Firefox),
        name: Words("The sharing indicator floats"),
        why: Words(
            "The small bar Firefox shows while your camera, microphone or screen is shared.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"firefox$"# title="Sharing Indicator$"
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[
            (
                Words("firefox"),
                Words("Firefox \u{2014} Sharing Indicator"),
            ),
            (
                Words("org.mozilla.firefox"),
                Words("Firefox \u{2014} Sharing Indicator"),
            ),
        ],
    },
    LibraryRule {
        app: Some(KnownApp::Wezterm),
        name: Words("Opens at the size it asks for"),
        why: Words(
            "WezTerm waits for a size of its own before it shows, so without this its window may never appear.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.wezfurlong\.wezterm$"#
    default-column-width {}
}"##,
        ),
        source: Origin::NiriDefaultConfig,
        examples: &[(Words("org.wezfurlong.wezterm"), Words("wezterm"))],
    },
    LibraryRule {
        app: Some(KnownApp::Steam),
        name: Words("Notifications float in the bottom-right corner"),
        why: Words(
            "Steam opens its notifications as windows of their own, in the middle of the screen.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id="steam" title=r#"^notificationtoasts_\d+_desktop$"#
    default-floating-position x=10 y=10 relative-to="bottom-right"
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(Words("steam"), Words("notificationtoasts_12_desktop"))],
    },
    LibraryRule {
        app: Some(KnownApp::Keepassxc),
        name: Words("Hidden from screen recordings"),
        why: Words("The password list shows as a black box to anyone watching a screen share."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.keepassxc\.KeePassXC$"#
    block-out-from "screencast"
}"##,
        ),
        source: Origin::NiriDefaultConfig,
        examples: &[(
            Words("org.keepassxc.KeePassXC"),
            Words("Passwords.kdbx - KeePassXC"),
        )],
    },
    LibraryRule {
        app: Some(KnownApp::Keepassxc),
        name: Words("The unlock dialog takes focus"),
        why: Words("So you can type the password as soon as a browser asks KeePassXC for one."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.keepassxc\.KeePassXC$"# title="^Unlock Database - KeePassXC$"
    open-focused true
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(
            Words("org.keepassxc.KeePassXC"),
            Words("Unlock Database - KeePassXC"),
        )],
    },
    LibraryRule {
        app: Some(KnownApp::Secrets),
        name: Words("Hidden from screen recordings"),
        why: Words("The password list shows as a black box to anyone watching a screen share."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.gnome\.World\.Secrets$"#
    block-out-from "screencast"
}"##,
        ),
        source: Origin::NiriDefaultConfig,
        examples: &[(Words("org.gnome.World.Secrets"), Words("Secrets"))],
    },
    LibraryRule {
        app: Some(KnownApp::Bitwarden),
        name: Words("Hidden from screen recordings"),
        why: Words("The vault shows as a black box to anyone watching a screen share."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^Bitwarden$"#
    block-out-from "screencast"
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[(Words("Bitwarden"), Words("Bitwarden"))],
    },
    LibraryRule {
        app: Some(KnownApp::OnePassword),
        name: Words("Hidden from screen recordings"),
        why: Words("The vault shows as a black box to anyone watching a screen share."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^1Password$"#
    block-out-from "screencast"
}"##,
        ),
        source: Origin::NiriCenter,
        examples: &[(Words("1Password"), Words("1Password"))],
    },
    LibraryRule {
        app: Some(KnownApp::Telegram),
        name: Words("The media viewer opens in a window"),
        why: Words(
            "Telegram shows photos and videos fullscreen; this keeps them in a window you can tile.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.telegram\.desktop$"# title="^Media viewer$"
    open-fullscreen false
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(Words("org.telegram.desktop"), Words("Media viewer"))],
    },
    LibraryRule {
        app: Some(KnownApp::Gimp),
        name: Words("The startup splash does not take focus"),
        why: Words("You can keep working in another window while GIMP starts."),
        kdl: Words(
            r##"window-rule {
    match app-id="^gimp" title="^GIMP Startup$"
    open-focused false
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(Words("gimp"), Words("GIMP Startup"))],
    },
    LibraryRule {
        app: Some(KnownApp::Pavucontrol),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^(org\.pulseaudio\.)?pavucontrol$"#
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[
            (Words("org.pulseaudio.pavucontrol"), Words("Volume Control")),
            (Words("pavucontrol"), Words("Volume Control")),
        ],
    },
    LibraryRule {
        app: Some(KnownApp::Pwvucontrol),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^com\.saivert\.pwvucontrol$"#
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[(
            Words("com.saivert.pwvucontrol"),
            Words("Pipewire Volume Control"),
        )],
    },
    LibraryRule {
        app: Some(KnownApp::GnomeCalculator),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^(org\.gnome\.Calculator|gnome-calculator)$"#
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[
            (Words("org.gnome.Calculator"), Words("Calculator")),
            (Words("gnome-calculator"), Words("Calculator")),
        ],
    },
    LibraryRule {
        app: Some(KnownApp::Kcalc),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^org\.kde\.kcalc$"#
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[(Words("org.kde.kcalc"), Words("KCalc"))],
    },
    LibraryRule {
        app: Some(KnownApp::NetworkConnections),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id="^nm-connection-editor$"
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[(Words("nm-connection-editor"), Words("Network Connections"))],
    },
    LibraryRule {
        app: Some(KnownApp::Blueman),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id="^blueman-manager$"
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[(Words("blueman-manager"), Words("Bluetooth Devices"))],
    },
    LibraryRule {
        app: Some(KnownApp::QtSettings),
        name: Words("Opens floating"),
        why: Words(SMALL_WINDOW),
        kdl: Words(
            r##"window-rule {
    match app-id="^qt[56]ct$"
    open-floating true
}"##,
        ),
        source: Origin::SharedConfigs,
        examples: &[
            (Words("qt5ct"), Words("Qt5 Configuration Tool")),
            (Words("qt6ct"), Words("Qt6 Configuration Tool")),
        ],
    },
    LibraryRule {
        app: Some(KnownApp::Mpv),
        name: Words("Variable refresh rate while it shows"),
        why: Words(
            "On a display set to on-demand VRR, video plays at its own frame rate while mpv is on screen.",
        ),
        kdl: Words(
            r##"window-rule {
    match app-id="^mpv$"
    variable-refresh-rate true
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(Words("mpv"), Words("video.mkv - mpv"))],
    },
    LibraryRule {
        app: Some(KnownApp::Obs),
        name: Words("Keeps a width its controls fit in"),
        why: Words("Narrower than this, OBS lays its controls over each other."),
        kdl: Words(
            r##"window-rule {
    match app-id=r#"^com\.obsproject\.Studio$"#
    min-width 876
}"##,
        ),
        source: Origin::NiriWiki,
        examples: &[(
            Words("com.obsproject.Studio"),
            Words("OBS 31.0.0 - Profile: Untitled"),
        )],
    },
    LibraryRule {
        app: Some(KnownApp::Fcitx),
        name: Words("The input window floats"),
        why: Words("Otherwise niri tiles Fcitx5's candidate list as a column of its own."),
        kdl: Words(
            r##"window-rule {
    match app-id="^fcitx$" title="^Fcitx5 Input Window$"
    open-floating true
}"##,
        ),
        source: Origin::NiriIssues(&[3917]),
        examples: &[(Words("fcitx"), Words("Fcitx5 Input Window"))],
    },
];

/// The library rules for one app, by its app ID: `niri-center.displays`
/// counts as niri-center.
pub fn for_app(app_id: &str) -> Vec<usize> {
    LIBRARY
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.app.is_some_and(|app| app.owns(app_id)))
        .map(|(index, _)| index)
        .collect()
}

/// The library's rules for a window's app that the config lacks, given
/// which it has ([`in_config`]).
pub fn missing(have: &[bool], app_id: &str) -> Vec<usize> {
    for_app(app_id)
        .into_iter()
        .filter(|&index| !have.get(index).copied().unwrap_or(false))
        .collect()
}

/// How many of the library's rules for a window's app the config lacks.
pub fn missing_for(have: &[bool], app_id: &str) -> usize {
    missing(have, app_id).len()
}

/// The apps the library has rules for, in the page's order: every window
/// first, then apps by name. Only apps installed here, unless `all`.
pub fn apps(system: &SystemData, all: bool) -> Vec<Option<KnownApp>> {
    let mut apps: Vec<Option<KnownApp>> = vec![None];
    for rule in &LIBRARY {
        if let Some(app) = rule.app
            && !apps.contains(&Some(app))
            && (all || app.installed(system))
        {
            apps.push(Some(app));
        }
    }
    // "every app" stays first, the apps follow by name
    if let Some(named) = apps.get_mut(1..) {
        named.sort_by_cached_key(|app| app.map(|app| app.name().to_lowercase()));
    }
    apps
}

/// What each library rule says, read as the config's rules are: two rules
/// that say the same are one however they are written.
pub struct Meanings(Vec<Option<RuleMeaning>>);

impl Default for Meanings {
    /// Every library rule, read.
    fn default() -> Self {
        Self(
            LIBRARY
                .iter()
                .map(|rule| {
                    let store = Store::from_text("library.kdl".into(), &rule.kdl).ok()?;
                    store.window_rule_rows().first().map(WindowRuleRow::meaning)
                })
                .collect(),
        )
    }
}

impl Meanings {
    /// Which of the library's rules the config's `rules` have turned on,
    /// by index.
    pub fn in_config(&self, rules: &[WindowRuleRow]) -> Vec<bool> {
        let have: HashSet<RuleMeaning> = rules
            .iter()
            .filter(|rule| !rule.disabled)
            .map(WindowRuleRow::meaning)
            .collect();
        self.0
            .iter()
            .map(|meaning| {
                meaning
                    .as_ref()
                    .is_some_and(|meaning| have.contains(meaning))
            })
            .collect()
    }
}

/// Add the library's rules at `marked` to `store`, as one change: into the
/// config itself, or into [`FILE`] beside it, which the config includes.
pub fn import(store: &mut Store, marked: &[usize], into_config: bool) -> Result<(), StoreError> {
    for rule in marked.iter().filter_map(|&index| LIBRARY.get(index)) {
        if into_config {
            store.add_window_rule_text(&rule.kdl)?;
        } else {
            store.add_to_include(FILE, &rule.kdl)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::niri::OpenWindow;
    use crate::rules::{Reach, reach};

    #[test]
    fn every_library_rule_reaches_the_windows_it_is_for() {
        for rule in &LIBRARY {
            let rows = Store::from_text("config.kdl".into(), &rule.kdl)
                .unwrap()
                .window_rule_rows();
            let [row] = rows.as_slice() else {
                panic!("{}: not one rule", rule.name);
            };
            for (app_id, title) in rule.examples {
                let window = OpenWindow {
                    id: 1,
                    app_id: (*app_id).to_string(),
                    title: (*title).to_string(),
                    floating: false,
                    focused: false,
                };
                assert_eq!(
                    reach(row, &window),
                    Reach::Reaches,
                    "{}: misses {app_id} {title:?}",
                    rule.name
                );
            }
            // every ID the app goes by has a window the rule is shown to reach
            if let Some(app) = rule.app {
                for id in app.ids() {
                    assert!(
                        rule.examples.iter().any(|(app_id, _)| app.owns(app_id)
                            && app_id.to_lowercase().starts_with(&id.to_lowercase())),
                        "{} ({}): no example for {id}",
                        rule.name,
                        app.name()
                    );
                }
            }
        }
    }

    #[test]
    fn a_window_is_matched_to_its_app_under_any_of_its_ids() {
        let floating = |app_id: &str| {
            for_app(app_id)
                .into_iter()
                .map(|index| LIBRARY[index].name.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(floating("pavucontrol"), ["Opens floating"]);
        assert_eq!(floating("org.pulseaudio.pavucontrol"), ["Opens floating"]);
        assert_eq!(floating("ORG.MOZILLA.FIREFOX").len(), 2);
        assert!(floating("org.example.Unknown").is_empty());
    }
}
