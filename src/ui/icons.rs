//! Shift's icon set: Phosphor Icons (regular weight, MIT licence, see
//! data/icons/shift/LICENSE-phosphor), bundled as `shift-*-symbolic` so GTK
//! recolours them with the text colour and they never shadow the desktop's
//! own icon names. Each constant keeps a freedesktop fallback.

pub const SERVER: &[&str] = &["shift-server-symbolic", "network-server-symbolic"];
pub const BACK: &[&str] = &["shift-back-symbolic", "go-previous-symbolic"];
pub const FORWARD: &[&str] = &["shift-forward-symbolic", "go-next-symbolic"];
pub const UP: &[&str] = &["shift-up-symbolic", "go-up-symbolic"];
pub const REFRESH: &[&str] = &["shift-refresh-symbolic", "view-refresh-symbolic"];
pub const FOLDER: &[&str] = &["shift-folder-symbolic", "folder-symbolic"];
pub const FOLDER_OPEN: &[&str] = &["shift-folder-open-symbolic", "folder-open-symbolic"];
pub const FOLDER_NEW: &[&str] = &["shift-folder-new-symbolic", "folder-new-symbolic"];
pub const FILE: &[&str] = &["shift-file-text-symbolic", "text-x-generic-symbolic"];
pub const ARCHIVE: &[&str] = &["shift-archive-symbolic", "package-x-generic-symbolic"];
pub const IMAGE: &[&str] = &["shift-image-symbolic", "image-x-generic-symbolic"];
pub const UPLOAD: &[&str] = &["shift-upload-symbolic", "document-send-symbolic"];
pub const DOWNLOAD: &[&str] = &["shift-download-symbolic", "folder-download-symbolic"];
pub const TRANSFERS: &[&str] = &["shift-transfers-symbolic", "folder-download-symbolic"];
pub const RENAME: &[&str] = &["shift-rename-symbolic", "document-edit-symbolic"];
pub const DELETE: &[&str] = &["shift-delete-symbolic", "user-trash-symbolic"];
pub const SHOW_HIDDEN: &[&str] = &["shift-show-hidden-symbolic", "view-reveal-symbolic"];
pub const HIDE_HIDDEN: &[&str] = &["shift-hide-hidden-symbolic", "view-conceal-symbolic"];
pub const COPY: &[&str] = &["shift-copy-symbolic", "edit-copy-symbolic"];
pub const CLEAR: &[&str] = &["shift-clear-symbolic", "edit-clear-all-symbolic"];
pub const TERMINAL: &[&str] = &["shift-terminal-symbolic", "utilities-terminal-symbolic"];
pub const SEPARATOR: &[&str] = &["shift-separator-symbolic", "pan-end-symbolic"];
pub const ADD: &[&str] = &["shift-add-symbolic", "list-add-symbolic"];
pub const SETTINGS: &[&str] = &["shift-settings-symbolic", "emblem-system-symbolic"];
pub const CLOSE: &[&str] = &["shift-close-symbolic", "window-close-symbolic"];
pub const QUICK: &[&str] = &["shift-quick-symbolic", "network-server-symbolic"];

/// First candidate present in the icon theme (bundled icons included).
pub fn pick(candidates: &[&'static str]) -> &'static str {
    if let Some(display) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        if let Some(found) = candidates.iter().find(|n| theme.has_icon(n)) {
            return found;
        }
    }
    candidates[0]
}
