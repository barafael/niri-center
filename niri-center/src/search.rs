//! The header search, over pages and settings, and the list of the app's
//! own keys the help card shows.

use crate::page::Page;
use crate::settings::Setting;
use crate::words::Words;

/// One search result: a page, or a setting on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    /// Shown beside the label: the page, or the setting's config path.
    pub detail: String,
    pub page: Page,
    /// The registry row to highlight on arrival.
    pub setting: Option<Setting>,
}

/// Search terms for pages beyond their names.
const PAGE_KEYWORDS: &[(Page, &str)] = &[
    (
        Page::Keybinds,
        "binds shortcuts hotkeys keys keyboard visualizer actions",
    ),
    (
        Page::Rules,
        "window rules match app-id float floating maximize opacity pick",
    ),
    (
        Page::Layout,
        "gaps focus ring border shadow colors gradient centering columns widths heights presets struts tabs insert hint blur",
    ),
    (
        Page::Displays,
        "monitors outputs screens arrange position mode scale refresh vrr rotate backdrop",
    ),
    (Page::Workspaces, "workspace empty back and forth"),
    (
        Page::Overview,
        "overview zoom backdrop alt tab switcher recent windows hotkey overlay",
    ),
    (
        Page::Animations,
        "curves easing bezier spring damping stiffness slowdown duration",
    ),
    (
        Page::Input,
        "keyboard touchpad mouse xkb layout variant options mod key tap scroll tablet touchscreen power",
    ),
    (Page::Gestures, "hot corners gestures"),
    (Page::Startup, "spawn-at-startup autostart launch programs"),
    (
        Page::Environment,
        "environment variables csd decorations title bars clipboard xwayland x11",
    ),
    (
        Page::Cursor,
        "cursor theme size hide pointer focus follows mouse warp",
    ),
    (Page::Screenshots, "screenshot path folder"),
    (Page::History, "history undo changes edits"),
    (
        Page::Files,
        "config includes backups snapshots restore sandbox",
    ),
    (Page::Windows, "open windows layers ipc toolbox"),
    (Page::Events, "live events stream focus workspaces"),
];

/// Every page and setting the search can find, each with its text in
/// lower case, so a keystroke only compares.
pub struct Index(Vec<(Entry, String)>);

impl Default for Index {
    fn default() -> Self {
        Self(entries())
    }
}

impl Index {
    /// Pages and settings whose text contains every query word.
    pub fn matches(&self, query: &str) -> impl Iterator<Item = &Entry> {
        let terms = Terms::of(query);
        self.0
            .iter()
            .filter(move |(_, haystack)| terms.all_in(haystack))
            .map(|(entry, _)| entry)
    }
}

/// A query's words, lower case: what a search looks for.
pub struct Terms(Vec<String>);

impl Terms {
    pub fn of(query: &str) -> Self {
        Self(query.split_whitespace().map(str::to_lowercase).collect())
    }

    /// Whether the lower-case `text` has every word; never for no words.
    pub fn all_in(&self, text: &str) -> bool {
        !self.0.is_empty() && self.0.iter().all(|word| text.contains(word.as_str()))
    }
}

fn entries() -> Vec<(Entry, String)> {
    let pages = Page::ALL.iter().map(|&page| {
        let keywords = PAGE_KEYWORDS
            .iter()
            .find(|(p, _)| *p == page)
            .map_or("", |(_, k)| *k);
        (
            Entry {
                label: page.label().to_string(),
                detail: "Page".to_string(),
                page,
                setting: None,
            },
            keywords.to_string(),
        )
    });
    let settings = Setting::ALL.iter().map(|&setting| {
        let spec = setting.spec();
        (
            Entry {
                label: spec.section.map_or_else(
                    || spec.label.to_string(),
                    |section| format!("{section}: {}", spec.label),
                ),
                detail: spec.path(),
                page: spec.page,
                setting: Some(setting),
            },
            format!("{} {}", spec.help, spec.page.label()),
        )
    });
    pages
        .chain(settings)
        .map(|(entry, extra)| {
            let haystack = format!("{} {} {extra}", entry.label, entry.detail).to_lowercase();
            (entry, haystack)
        })
        .collect()
}

/// A group of lines on the help card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpGroup {
    MovingAround,
    Changes,
    Windows,
}

impl HelpGroup {
    pub const ALL: [Self; 3] = [Self::MovingAround, Self::Changes, Self::Windows];

    pub const fn heading(self) -> &'static str {
        match self {
            Self::MovingAround => "Moving around",
            Self::Changes => "Changes",
            Self::Windows => "Windows",
        }
    }
}

/// One line on the help card.
pub struct HelpLine {
    pub group: HelpGroup,
    /// The keys, as people read them.
    pub keys: Words,
    pub what: Words,
    /// The same keys as niri writes them in a bind: niri takes a bound key
    /// before the app sees it.
    pub binds: &'static [Words],
}

/// The app's own keys, as the help card lists them.
pub const HELP: [HelpLine; 14] = [
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Ctrl+PgUp, Ctrl+PgDn"),
        what: Words("Previous or next section"),
        binds: &[Words("Ctrl+Page_Up"), Words("Ctrl+Page_Down")],
    },
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Ctrl+1 to Ctrl+0"),
        what: Words("Go to the first ten sections"),
        binds: &[
            Words("Ctrl+1"),
            Words("Ctrl+2"),
            Words("Ctrl+3"),
            Words("Ctrl+4"),
            Words("Ctrl+5"),
            Words("Ctrl+6"),
            Words("Ctrl+7"),
            Words("Ctrl+8"),
            Words("Ctrl+9"),
            Words("Ctrl+0"),
        ],
    },
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Ctrl+F, Ctrl+K, or type"),
        what: Words("Find a setting, key or action"),
        binds: &[Words("Ctrl+F"), Words("Ctrl+K")],
    },
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Up, Down"),
        what: Words("Move in the search results"),
        binds: &[],
    },
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Esc"),
        what: Words("Close search or this list, or go back"),
        binds: &[],
    },
    HelpLine {
        group: HelpGroup::MovingAround,
        keys: Words("Alt+Left"),
        what: Words("Back up one level in Window rules"),
        binds: &[Words("Alt+Left")],
    },
    HelpLine {
        group: HelpGroup::Changes,
        keys: Words("Ctrl+Z"),
        what: Words("Undo the latest change"),
        binds: &[Words("Ctrl+Z")],
    },
    HelpLine {
        group: HelpGroup::Changes,
        keys: Words("Ctrl+S"),
        what: Words("Save now instead of a moment later"),
        binds: &[Words("Ctrl+S")],
    },
    HelpLine {
        group: HelpGroup::Changes,
        keys: Words("Enter or Esc"),
        what: Words("While trying a display change: go back"),
        binds: &[],
    },
    HelpLine {
        group: HelpGroup::Windows,
        keys: Words("Ctrl+B"),
        what: Words("Fold the sidebar to icons, or open it"),
        binds: &[Words("Ctrl+B")],
    },
    HelpLine {
        group: HelpGroup::Windows,
        keys: Words("Ctrl+."),
        what: Words("Show or hide the config beside the page"),
        binds: &[Words("Ctrl+Period")],
    },
    HelpLine {
        group: HelpGroup::Windows,
        keys: Words("Ctrl+Shift+N"),
        what: Words("Open the config as a window of its own"),
        binds: &[Words("Ctrl+Shift+N")],
    },
    HelpLine {
        group: HelpGroup::Windows,
        keys: Words("Ctrl+W"),
        what: Words("In that window: put it back"),
        binds: &[Words("Ctrl+W")],
    },
    HelpLine {
        group: HelpGroup::Windows,
        keys: Words("F1, Ctrl+?"),
        what: Words("This list"),
        binds: &[Words("F1"), Words("Ctrl+Question")],
    },
];
