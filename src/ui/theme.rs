//! Shift design system: one consistent look on every desktop.
//!
//! Palette and radii come from our own apps (quick-send `QsPalette`, Kota
//! cards); layout ideas from Termius. Light/dark follows the desktop (or the
//! user's choice), the accent follows the desktop accent when the XDG portal
//! provides one, else Shift blue. Fonts and icons stay the system's.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use gtk4::prelude::*;

pub type Rgb = (f64, f64, f64);

struct Palette {
    bg: &'static str,
    sidebar: &'static str,
    card: &'static str,
    card_hover: &'static str,
    input: &'static str,
    border: &'static str,
    text: &'static str,
    sub: &'static str,
    faint: &'static str,
    chip: &'static str,
    danger: &'static str,
    success: &'static str,
    warning: &'static str,
    accent: &'static str,
}

// quick-send QsPalette.darkPalette / .light, extended for a desktop app.
const DARK: Palette = Palette {
    bg: "#111214",
    sidebar: "#17181b",
    card: "#1b1c20",
    card_hover: "#222328",
    input: "#141518",
    border: "#2a2b30",
    text: "#f2f2f2",
    sub: "#a3a3a3",
    faint: "#26272c",
    chip: "#222327",
    danger: "#ff6b6b",
    success: "#3ccf8e",
    warning: "#f5a524",
    accent: "#3b82f6",
};

const LIGHT: Palette = Palette {
    bg: "#f2f3f7",
    sidebar: "#e9ebf0",
    card: "#ffffff",
    card_hover: "#f7f8fb",
    input: "#ffffff",
    border: "#e1e3ea",
    text: "#17181c",
    sub: "#6e7079",
    faint: "#e3e4ea",
    chip: "#ecedf2",
    danger: "#e5484d",
    success: "#1a9f5b",
    warning: "#c77c02",
    accent: "#3478f6",
};

fn rgb_css((r, g, b): Rgb) -> String {
    format!("rgb({},{},{})", (r * 255.0).round(), (g * 255.0).round(), (b * 255.0).round())
}

fn css(dark: bool, accent: Option<Rgb>) -> String {
    let p = if dark { &DARK } else { &LIGHT };
    let accent = accent.map(rgb_css).unwrap_or_else(|| p.accent.to_string());
    let defs = format!(
        "@define-color shift_bg {};\n@define-color shift_sidebar {};\n@define-color shift_card {};\n\
@define-color shift_card_hover {};\n@define-color shift_input {};\n@define-color shift_border {};\n\
@define-color shift_text {};\n@define-color shift_sub {};\n@define-color shift_faint {};\n\
@define-color shift_chip {};\n@define-color shift_danger {};\n@define-color shift_success {};\n\
@define-color shift_warning {};\n@define-color shift_accent {};\n\
@define-color accent_color {accent};\n@define-color accent_bg_color {accent};\n\
@define-color theme_selected_bg_color {accent};\n",
        p.bg, p.sidebar, p.card, p.card_hover, p.input, p.border, p.text, p.sub, p.faint,
        p.chip, p.danger, p.success, p.warning, accent
    );
    defs + RULES
}

const RULES: &str = r#"
/* ---- base ---- */
window.shift, window.shift.background, .shift-dialog, .shift-dialog.background {
    background-color: @shift_bg;
    color: @shift_text;
}
window.shift headerbar, .shift-dialog headerbar {
    background-color: @shift_sidebar;
    background-image: none;
    box-shadow: none;
    border-bottom: 1px solid @shift_border;
    min-height: 42px;
    color: @shift_text;
}
.dim, .dim-label { color: @shift_sub; opacity: 1; }
.caption { font-size: 0.88em; }
.heading { font-weight: 700; }
.title-1 { font-size: 1.6em; font-weight: 700; }
.page-title { font-size: 1.45em; font-weight: 700; }
.section-title { font-size: 0.82em; font-weight: 700; color: @shift_sub; letter-spacing: 0.04em; }
.mono { font-family: monospace; }

/* ---- sidebar (Termius / quick-send nav) ---- */
.sidebar {
    background-color: @shift_sidebar;
    border-right: 1px solid @shift_border;
    padding: 14px 10px;
}
.brand { font-size: 1.15em; font-weight: 700; }
button.nav-item {
    background: none;
    border: none;
    box-shadow: none;
    border-radius: 10px;
    padding: 8px 10px;
    color: @shift_sub;
    font-weight: 500;
    transition: background-color 120ms ease, color 120ms ease;
}
button.nav-item:hover { background-color: @shift_faint; color: @shift_text; }
button.nav-item:checked { background-color: alpha(@shift_accent, 0.16); color: @shift_text; font-weight: 600; }
button.nav-item:checked image { color: @shift_accent; }
button.nav-item:disabled { opacity: 0.4; }
menubutton.nav-item > button {
    background: none;
    border: none;
    box-shadow: none;
    border-radius: 10px;
    padding: 8px 10px;
    color: @shift_sub;
}
menubutton.nav-item > button:hover, menubutton.nav-item > button:checked { background-color: @shift_faint; color: @shift_text; }

/* ---- cards (Kota) ---- */
.card {
    background-color: @shift_card;
    border: 1px solid @shift_border;
    border-radius: 16px;
}
.card.padded { padding: 16px; }
.stat-value { font-size: 1.35em; font-weight: 700; }

flowbox.host-grid > flowboxchild { padding: 0; background: none; border-radius: 16px; outline: none; }
flowbox.host-grid > flowboxchild:focus-visible > .host-card { border-color: @shift_accent; }
.host-card {
    padding: 14px;
    transition: background-color 120ms ease, border-color 120ms ease;
}
.host-card:hover { background-color: @shift_card_hover; border-color: alpha(@shift_accent, 0.55); }
.host-tile {
    background-color: alpha(@shift_accent, 0.15);
    color: @shift_accent;
    border-radius: 12px;
    min-width: 40px;
    min-height: 40px;
}
.host-tile image { -gtk-icon-size: 18px; }
.host-tile.ftp { background-color: alpha(@shift_warning, 0.15); color: @shift_warning; }
.host-tile.ftps { background-color: alpha(@shift_success, 0.15); color: @shift_success; }

.chip {
    background-color: @shift_chip;
    color: @shift_sub;
    border-radius: 999px;
    padding: 2px 10px;
    font-size: 0.82em;
    font-weight: 600;
}
.status-dot { min-width: 8px; min-height: 8px; border-radius: 999px; background-color: @shift_sub; }
.status-dot.online { background-color: @shift_success; box-shadow: 0 0 0 3px alpha(@shift_success, 0.18); }
.text-success { color: @shift_success; }
.text-danger { color: @shift_danger; }

/* ---- inputs (quick-send) ---- */
window.shift entry, window.shift passwordentry, window.shift searchentry, window.shift spinbutton,
window.shift dropdown > button, .shift-dialog entry, .shift-dialog passwordentry, .shift-dialog dropdown > button {
    background-color: @shift_input;
    background-image: none;
    border: 1px solid @shift_border;
    border-radius: 10px;
    box-shadow: none;
    min-height: 34px;
    color: @shift_text;
    outline: 0 solid transparent;
}
window.shift entry:focus-within, window.shift passwordentry:focus-within, window.shift searchentry:focus-within,
.shift-dialog entry:focus-within, .shift-dialog passwordentry:focus-within {
    border-color: @shift_accent;
    outline: 3px solid alpha(@shift_accent, 0.22);
    outline-offset: -1px;
}
entry.error, passwordentry.error { border-color: @shift_danger; }
.omnibox { min-height: 42px; border-radius: 12px; font-size: 1.02em; }

/* ---- buttons ---- */
window.shift button, .shift-dialog button { border-radius: 10px; }
window.shift button.flat:hover, .shift-dialog button.flat:hover { background-color: @shift_faint; }
button.suggested-action, button.accent {
    background-color: @shift_accent;
    background-image: none;
    color: white;
    border: none;
    box-shadow: none;
    font-weight: 600;
    padding: 6px 16px;
}
button.suggested-action:hover, button.accent:hover { background-color: shade(@shift_accent, 1.08); }
button.suggested-action:active, button.accent:active { background-color: shade(@shift_accent, 0.9); }
button.soft {
    background-color: @shift_chip;
    background-image: none;
    border: none;
    box-shadow: none;
    color: @shift_text;
    font-weight: 600;
    padding: 6px 14px;
}
button.soft:hover { background-color: @shift_faint; }
button.danger {
    background-color: alpha(@shift_danger, 0.12);
    background-image: none;
    color: @shift_danger;
    border: none;
    box-shadow: none;
    font-weight: 600;
    padding: 6px 14px;
}
button.danger:hover { background-color: alpha(@shift_danger, 0.2); }
check:checked, radio:checked { background-color: @shift_accent; background-image: none; border-color: @shift_accent; color: white; }
switch:checked { background-color: @shift_accent; }

/* ---- side editor panel (Termius "Edit host") ---- */
.editor-panel {
    background-color: @shift_sidebar;
    border-left: 1px solid @shift_border;
}
.form-card { padding: 4px 14px; }
.form-row { padding: 8px 0; border-bottom: 1px solid alpha(@shift_border, 0.7); }
.form-row:last-child { border-bottom: none; }
expander title { color: @shift_sub; font-weight: 700; font-size: 0.85em; }

/* ---- files (Termius SFTP) ---- */
.breadcrumb {
    background-color: @shift_card;
    border: 1px solid @shift_border;
    border-radius: 10px;
    padding: 0 4px;
}
.breadcrumb button { padding: 2px 6px; min-height: 28px; }
treeview.data-table { background-color: @shift_card; color: @shift_text; }
treeview.data-table:hover { background-color: alpha(@shift_text, 0.03); }
treeview.data-table:selected { background-color: alpha(@shift_accent, 0.22); color: @shift_text; }
treeview.data-table header button {
    background: @shift_card;
    border-radius: 0;
    border: none;
    border-bottom: 1px solid @shift_border;
    box-shadow: none;
    color: @shift_sub;
    font-size: 0.82em;
    font-weight: 700;
    padding: 8px 10px;
}
.files-card { border-radius: 16px; }

/* ---- log ---- */
textview.log-view, textview.log-view text { background-color: @shift_card; color: @shift_text; }

/* ---- popovers / notifications ---- */
window.shift popover > contents, .shift-dialog popover > contents {
    background-color: @shift_card;
    border: 1px solid @shift_border;
    border-radius: 12px;
    color: @shift_text;
}
window.shift popover > arrow { background-color: @shift_card; border-color: @shift_border; }
.app-notification {
    background-color: @shift_card_hover;
    color: @shift_text;
    border: 1px solid @shift_border;
    border-radius: 12px;
    padding: 6px 8px 6px 14px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.25);
}
notebook > header { background: none; border: none; }
"#;

/// Reads the desktop's dark preference and re-renders the stylesheet.
pub struct Theme {
    provider: gtk4::CssProvider,
    settings: gtk4::Settings,
    mode: RefCell<String>,
    accent: Cell<Option<Rgb>>,
    system_theme: RefCell<String>,
    system_prefer_dark: bool,
}

/// Appearance choice is kept in ~/.config/shift/appearance (system|light|dark).
fn mode_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("shift").join("appearance"))
}

fn load_mode() -> String {
    mode_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|m| matches!(m.as_str(), "system" | "light" | "dark"))
        .unwrap_or_else(|| "system".into())
}

fn save_mode(mode: &str) {
    if let Some(p) = mode_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, mode);
    }
}

fn enum_nick(obj: &impl IsA<glib::Object>, prop: &str) -> Option<String> {
    obj.find_property(prop)?;
    let value = obj.property_value(prop);
    glib::EnumValue::from_value(&value).map(|(_, v)| v.nick().to_string())
}

impl Theme {
    pub fn install(display: &gtk4::gdk::Display) -> Rc<Theme> {
        let settings = gtk4::Settings::for_display(display);
        let system_theme = std::env::var("SHIFT_HOST_GTK_THEME")
            .ok()
            .or_else(|| settings.gtk_theme_name().map(|s| s.to_string()))
            .unwrap_or_default();
        let theme = Rc::new(Theme {
            provider: gtk4::CssProvider::new(),
            settings: settings.clone(),
            mode: RefCell::new(load_mode()),
            accent: Cell::new(None),
            system_theme: RefCell::new(system_theme),
            system_prefer_dark: settings.is_gtk_application_prefer_dark_theme(),
        });
        // Above the GTK theme and settings, below ~/.config/gtk-4.0/gtk.css.
        gtk4::style_context_add_provider_for_display(display, &theme.provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let t = Rc::clone(&theme);
        settings.connect_notify_local(Some("gtk-interface-color-scheme"), move |_, _| t.reload());
        let t = Rc::clone(&theme);
        settings.connect_gtk_theme_name_notify(move |s| {
            // The desktop changed its theme (e.g. KDE Breeze -> Breeze-Dark):
            // remember it for dark detection, keep our base theme.
            let name = s.gtk_theme_name().map(|n| n.to_string()).unwrap_or_default();
            if name != "Adwaita" {
                *t.system_theme.borrow_mut() = name;
                t.reload();
            }
        });

        let t = Rc::clone(&theme);
        crate::ui::accent::watch(move |rgb| {
            t.accent.set(rgb);
            t.reload();
        });

        theme.reload();
        theme
    }

    fn system_dark(&self) -> bool {
        match enum_nick(&self.settings, "gtk-interface-color-scheme").as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => self.system_prefer_dark || self.system_theme.borrow().to_lowercase().contains("dark"),
        }
    }

    pub fn is_dark(&self) -> bool {
        match self.mode.borrow().as_str() {
            "dark" => true,
            "light" => false,
            _ => self.system_dark(),
        }
    }

    pub fn mode(&self) -> String {
        self.mode.borrow().clone()
    }

    pub fn set_mode(&self, mode: &str) {
        *self.mode.borrow_mut() = mode.to_string();
        save_mode(mode);
        self.reload();
    }

    fn reload(&self) {
        let dark = self.is_dark();
        // GTK's built-in theme is the neutral base we style on top of, so the
        // design is identical on GNOME, KDE, XFCE...; native dialogs (file
        // chooser) still use the matching light/dark variant.
        if self.settings.gtk_theme_name().as_deref() != Some("Adwaita") {
            self.settings.set_gtk_theme_name(Some("Adwaita"));
        }
        self.settings.set_gtk_application_prefer_dark_theme(dark);
        self.provider.load_from_string(&css(dark, self.accent.get()));
    }
}
