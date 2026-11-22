//! How the app looks: light or dark like the system, in the system's UI font, with walroos'
//! magenta as the primary color.

use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use gpui_kit::component::theme::Theme;
use gpui_kit::*;

/// Follows the system's light/dark setting. Changing the mode reloads the theme, so the
/// walroos touches go back on after every change.
pub fn follow_system(window: &mut Window, cx: &mut App) {
    Theme::sync_system_appearance(Some(window), cx);
    walroos_look(cx);
    window
        .observe_window_appearance(|window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            walroos_look(cx);
        })
        .detach();
}

fn walroos_look(cx: &mut App) {
    Theme::update(cx, |t| {
        brand(t);
        if let Some(family) = system_ui_font() {
            t.font_family = family.clone().into();
        }
        // A little longer than gpui-component's defaults (120/180/280 ms), closer to Material 3's
        // short and medium durations: menus and dialogs ease in instead of popping.
        t.motion.duration_fast = Duration::from_millis(150);
        t.motion.duration_normal = Duration::from_millis(250);
        t.motion.duration_slow = Duration::from_millis(350);
    });
}

/// The walroos primary: magenta, AA-contrast with its text in both modes (walroos' AGENTS.md, Brand).
fn brand(t: &mut Theme) {
    let (primary, on_primary): (Hsla, Hsla) = if t.is_dark() {
        (rgb(0xEE5A96).into(), rgb(0x1A0510).into())
    } else {
        (rgb(0xC21F65).into(), rgb(0xFFFFFF).into())
    };
    t.primary = primary;
    t.primary_hover = primary.opacity(0.9);
    t.primary_active = primary.opacity(0.8);
    t.primary_foreground = on_primary;
    t.button_primary = t.primary;
    t.button_primary_hover = t.primary_hover;
    t.button_primary_active = t.primary_active;
    t.button_primary_foreground = on_primary;
    t.progress_bar = primary;
    t.ring = primary;
}

/// The desktop's UI font family, found once. GPUI doesn't read it on Linux: its system font is
/// a fixed name with a fallback list. The desktop's choice is the GTK/GNOME `font-name` setting
/// (what GNOME, nwg-look and most Wayland setups write), resolved to an installed family by
/// fontconfig; without it, fontconfig's sans-serif.
// ponytail: Linux only, and read at startup: changing the system font needs a restart. GPUI picks
// a weight among a family's installed files and doesn't set a variable font's weight axis, so a
// family installed only as a variable font draws every weight like its default instance.
fn system_ui_font() -> Option<&'static String> {
    static FONT: OnceLock<Option<String>> = OnceLock::new();
    FONT.get_or_init(|| {
        if !cfg!(target_os = "linux") {
            return None;
        }
        let setting = output("gsettings", &["get", "org.gnome.desktop.interface", "font-name"]);
        let pattern = setting
            .as_deref()
            .and_then(font_name_family)
            .unwrap_or("sans-serif".into());
        output("fc-match", &["-f", "%{family[0]}", &pattern]).filter(|f| !f.is_empty())
    })
    .as_ref()
}

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 'Google Sans 10' → Google Sans: gsettings quotes the value and ends it with the size.
fn font_name_family(setting: &str) -> Option<String> {
    let name = setting.trim().trim_matches('\'');
    let family = match name.rsplit_once(' ') {
        Some((family, size)) if size.parse::<f32>().is_ok() => family,
        _ => name,
    };
    (!family.is_empty()).then(|| family.to_string())
}

#[cfg(test)]
mod tests {
    use super::font_name_family;

    #[test]
    fn reads_the_family_out_of_gsettings() {
        assert_eq!(
            font_name_family("'Google Sans 10'\n").as_deref(),
            Some("Google Sans")
        );
        assert_eq!(
            font_name_family("'Noto Sans Medium 10.5'").as_deref(),
            Some("Noto Sans Medium")
        );
        assert_eq!(font_name_family("'Cantarell'").as_deref(), Some("Cantarell"));
        assert_eq!(font_name_family("''"), None);
    }
}
