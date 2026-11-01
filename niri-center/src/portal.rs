//! The desktop's side of things, through xdg-desktop-portal on the session
//! bus: the file chooser behind Browse buttons, and the color scheme and
//! contrast the desktop asks for.
//!
//! On niri the chooser usually comes from the gtk portal, or from nautilus
//! for the gnome one; when neither answers, the text field next to the
//! button still works and the status line says why the dialog did not open.
//! Interfaces as documented at
//! <https://flatpak.github.io/xdg-desktop-portal/docs/>.

/// The light or dark the desktop asks apps for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    NoPreference,
    Dark,
    Light,
}

/// A change of what the desktop asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Scheme(Scheme),
    /// Whether it asks for more contrast.
    HighContrast(bool),
}

#[cfg(not(target_arch = "wasm32"))]
mod bus {
    use std::collections::HashMap;

    use futures_util::StreamExt;
    use zbus::proxy::{Builder, CacheProperties};
    use zbus::{Connection, OwnedObjectPath, OwnedVariant, Proxy, Variant};

    use super::{Appearance, Scheme};

    const PORTAL: &str = "org.freedesktop.portal.Desktop";
    const DESKTOP: &str = "/org/freedesktop/portal/desktop";
    const APPEARANCE: &str = "org.freedesktop.appearance";

    /// The portal interfaces the app calls.
    #[derive(Debug, Clone, Copy)]
    enum Interface {
        Settings,
        FileChooser,
        /// One call's answer, at the handle the call returns.
        Request,
    }

    impl Interface {
        const fn as_str(self) -> &'static str {
            match self {
                Self::Settings => "org.freedesktop.portal.Settings",
                Self::FileChooser => "org.freedesktop.portal.FileChooser",
                Self::Request => "org.freedesktop.portal.Request",
            }
        }
    }

    /// `interface` of the portal object at `path`. Request objects exist only
    /// for one call, so nothing is read from them ahead.
    async fn proxy<'path>(
        connection: &Connection,
        path: &'path str,
        interface: Interface,
    ) -> zbus::Result<Proxy<'path>> {
        Builder::<Proxy>::new(connection)
            .destination(PORTAL)
            .path(path)
            .interface(interface.as_str())
            .cache_properties(CacheProperties::No)
            .build()
            .await
    }

    /// The number inside `variant`: the portal's deprecated `Read` wraps it
    /// in a second variant.
    fn number(variant: &Variant<'_>) -> Option<u32> {
        if let Variant::U32(number) = variant {
            return Some(*number);
        }
        if let Variant::Variant(inner) = variant {
            return number(inner);
        }
        None
    }

    /// `org.freedesktop.appearance` `color-scheme`: "0: No preference,
    /// 1: Prefer dark appearance, 2: Prefer light appearance".
    const fn scheme_of(value: u32) -> Scheme {
        match value {
            1 => Scheme::Dark,
            2 => Scheme::Light,
            _ => Scheme::NoPreference,
        }
    }

    /// One `org.freedesktop.appearance` setting; `None` without an answer.
    async fn appearance(key: &str) -> Option<u32> {
        let connection = Connection::session().await.ok()?;
        let settings = proxy(&connection, DESKTOP, Interface::Settings)
            .await
            .ok()?;
        // ReadOne came with version 2 of the interface
        let read: zbus::Result<OwnedVariant> = settings.call("ReadOne", &(APPEARANCE, key)).await;
        let value = match read {
            Ok(value) => value,
            Err(_) => settings.call("Read", &(APPEARANCE, key)).await.ok()?,
        };
        number(&value)
    }

    pub async fn scheme() -> Scheme {
        appearance("color-scheme")
            .await
            .map_or(Scheme::NoPreference, scheme_of)
    }

    /// `contrast`: "0: No preference (normal contrast), 1: Higher contrast".
    pub async fn high_contrast() -> bool {
        appearance("contrast").await == Some(1)
    }

    pub async fn watch(mut each: impl FnMut(Appearance) -> bool) {
        let Ok(connection) = Connection::session().await else {
            return;
        };
        let Ok(settings) = proxy(&connection, DESKTOP, Interface::Settings).await else {
            return;
        };
        let Ok(mut changes) = settings
            .receive_signal_with_args("SettingChanged", &[(0, APPEARANCE)])
            .await
        else {
            return;
        };
        while let Some(change) = changes.next().await {
            let Ok((_, key, value)) = change
                .body()
                .deserialize::<(String, String, OwnedVariant)>()
            else {
                continue;
            };
            let appearance = match (key.as_str(), number(&value)) {
                ("color-scheme", Some(value)) => Appearance::Scheme(scheme_of(value)),
                ("contrast", Some(value)) => Appearance::HighContrast(value == 1),
                _ => continue,
            };
            if !each(appearance) {
                break;
            }
        }
    }

    /// What the chooser answered: the file's URI, or none when cancelled.
    pub async fn choose(title: &str, folder: bool) -> Result<Option<String>, String> {
        let connection = Connection::session()
            .await
            .map_err(|error| error.to_string())?;
        // the request's object is known before the call, so its answer is
        // listened for first and cannot come before it
        let sender = connection
            .unique_name()
            .ok_or_else(|| "no name on the session bus".to_string())?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        // one token per request, so two choosers open at once answer apart
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let token = format!("niri_center_{}_{started}", std::process::id());
        let expected = format!("{DESKTOP}/request/{sender}/{token}");
        let request = proxy(&connection, &expected, Interface::Request)
            .await
            .map_err(|error| error.to_string())?;
        let mut answers = request
            .receive_signal("Response")
            .await
            .map_err(|error| error.to_string())?;

        let chooser = proxy(&connection, DESKTOP, Interface::FileChooser)
            .await
            .map_err(|error| error.to_string())?;
        let options = HashMap::from([
            ("handle_token", Variant::from(token.as_str())),
            ("modal", Variant::from(true)),
            ("directory", Variant::from(folder)),
        ]);
        let handle: OwnedObjectPath = chooser
            .call("OpenFile", &("", title, options))
            .await
            .map_err(|error| error.to_string())?;
        // portals before the handle was predictable choose their own
        if handle.as_str() != expected {
            let request = proxy(&connection, handle.as_str(), Interface::Request)
                .await
                .map_err(|error| error.to_string())?;
            answers = request
                .receive_signal("Response")
                .await
                .map_err(|error| error.to_string())?;
        }
        let answer = answers
            .next()
            .await
            .ok_or_else(|| "the portal closed without an answer".to_string())?;
        let (response, results) = answer
            .body()
            .deserialize::<(u32, HashMap<String, OwnedVariant>)>()
            .map_err(|error| error.to_string())?;
        // "0: Success, 1: The user cancelled the interaction, 2: The user
        // interaction was ended in some other way"
        match response {
            0 => Ok(results
                .get("uris")
                .and_then(|uris| Vec::<String>::try_from(Variant::clone(uris)).ok())
                .and_then(|uris| uris.into_iter().next())),
            1 => Ok(None),
            _ => Err("the chooser ended without a choice".to_string()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_setting_is_read_through_either_wrapping() {
            // ReadOne answers with the value; the older Read wraps it again
            assert_eq!(number(&Variant::from(1u32)), Some(1));
            let wrapped = Variant::new(Variant::new(Variant::new(2u32)));
            assert_eq!(number(&wrapped), Some(2));
            assert_eq!(number(&Variant::from("dark")), None);
        }

        #[test]
        fn color_scheme_numbers_are_the_portals() {
            assert_eq!(scheme_of(0), Scheme::NoPreference);
            assert_eq!(scheme_of(1), Scheme::Dark);
            assert_eq!(scheme_of(2), Scheme::Light);
            assert_eq!(scheme_of(7), Scheme::NoPreference);
        }
    }
}

/// The light or dark the desktop asks for; no preference without a portal.
#[cfg(not(target_arch = "wasm32"))]
pub async fn scheme() -> Scheme {
    bus::scheme().await
}

/// Ask the user for a file, or a folder when `folder`; `Ok(None)` when they
/// cancelled. The path comes back with the home folder as `~`.
#[cfg(not(target_arch = "wasm32"))]
pub async fn choose(folder: bool) -> Result<Option<String>, String> {
    let title = if folder {
        "Choose a folder"
    } else {
        "Choose a file"
    };
    match bus::choose(title, folder).await {
        Ok(uri) => Ok(uri
            .as_deref()
            .and_then(file_path)
            .map(|path| crate::store::tilde(std::path::Path::new(&path)))),
        Err(failure) => Err(format!(
            "The file chooser did not open ({failure}). It needs xdg-desktop-portal with a file chooser, such as the gtk portal. You can type the path instead."
        )),
    }
}

/// The browser demo has no file chooser to ask.
#[cfg(target_arch = "wasm32")]
pub async fn choose(_folder: bool) -> Result<Option<String>, String> {
    Err("The file chooser opens in the desktop app. In this demo, type the path.".to_string())
}

/// `file:///home/me/My%20Pictures` → `/home/me/My Pictures`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn file_path(uri: &str) -> Option<String> {
    let encoded = uri.strip_prefix("file://")?;
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%'
            && let Some(decoded) = encoded
                .get(index + 1..index + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(decoded);
            index += 3;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Whether the desktop asks apps for more contrast (GNOME's high-contrast
/// switch sets it). No answer counts as no.
#[cfg(not(target_arch = "wasm32"))]
pub async fn high_contrast() -> bool {
    bus::high_contrast().await
}

/// The browser's `prefers-contrast: more`, for the demo.
#[cfg(target_arch = "wasm32")]
pub async fn high_contrast() -> bool {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-contrast: more)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}

/// Hand every later change of the desktop's color scheme or contrast to
/// `each`, until `each` says stop.
#[cfg(not(target_arch = "wasm32"))]
pub async fn watch_appearance(each: impl FnMut(Appearance) -> bool) {
    bus::watch(each).await;
}

/// The browser's preferences are read once, at start.
#[cfg(target_arch = "wasm32")]
pub async fn watch_appearance(_each: impl FnMut(Appearance) -> bool) {}

#[cfg(test)]
mod tests {
    #[test]
    fn portal_uris_become_paths() {
        assert_eq!(
            super::file_path("file:///home/me/My%20Pictures").as_deref(),
            Some("/home/me/My Pictures")
        );
        assert_eq!(super::file_path("https://example.org"), None);
    }
}
