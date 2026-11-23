//! Standard freedesktop icon names, resolved against the user's icon theme.
//!
//! Each constant lists candidates in order of preference. `pick` returns the
//! first one the active icon theme (Adwaita, Breeze, elementary, Papirus...)
//! actually provides, so icons always follow the desktop's look.

pub const SERVER: &[&str] = &["network-server-symbolic"];
pub const BACK: &[&str] = &["go-previous-symbolic"];
pub const FORWARD: &[&str] = &["go-next-symbolic"];
pub const UP: &[&str] = &["go-up-symbolic"];
pub const REFRESH: &[&str] = &["view-refresh-symbolic"];
pub const FOLDER: &[&str] = &["folder-symbolic"];
pub const FOLDER_OPEN: &[&str] = &["folder-open-symbolic", "document-open-symbolic"];
pub const FOLDER_NEW: &[&str] = &["folder-new-symbolic", "list-add-symbolic"];
pub const FILE: &[&str] = &["text-x-generic-symbolic"];
pub const ARCHIVE: &[&str] = &["package-x-generic-symbolic", "text-x-generic-symbolic"];
pub const IMAGE: &[&str] = &["image-x-generic-symbolic", "text-x-generic-symbolic"];
pub const UPLOAD: &[&str] = &["document-send-symbolic", "go-up-symbolic"];
pub const DOWNLOAD: &[&str] = &["folder-download-symbolic", "document-save-symbolic"];
pub const TRANSFERS: &[&str] = &["folder-download-symbolic", "document-save-symbolic"];
pub const RENAME: &[&str] = &["document-edit-symbolic", "edit-rename-symbolic"];
pub const DELETE: &[&str] = &["user-trash-symbolic", "edit-delete-symbolic"];
pub const SHOW_HIDDEN: &[&str] = &["view-reveal-symbolic", "view-visible-symbolic"];
pub const HIDE_HIDDEN: &[&str] = &["view-conceal-symbolic", "view-hidden-symbolic"];
pub const COPY: &[&str] = &["edit-copy-symbolic"];
pub const CLEAR: &[&str] = &["edit-clear-all-symbolic", "edit-clear-symbolic"];
pub const TERMINAL: &[&str] = &["utilities-terminal-symbolic"];
pub const SEPARATOR: &[&str] = &["pan-end-symbolic", "go-next-symbolic"];

/// First candidate present in the current icon theme, else the first candidate
/// (GTK then shows its own "missing" fallback rather than crashing).
pub fn pick(candidates: &[&'static str]) -> &'static str {
    if let Some(display) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        if let Some(found) = candidates.iter().find(|n| theme.has_icon(n)) {
            return found;
        }
    }
    candidates[0]
}
