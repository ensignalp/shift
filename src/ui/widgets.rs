//! Small plain-GTK4 building blocks that replace the libadwaita widgets.
//! They only use stock GTK widgets and style classes every GTK4 theme
//! (Default/Adwaita, Breeze, Greybird, Yaru, ...) knows about.

use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;

use crate::ui::icons;

/// Flat icon-only button with a tooltip.
pub fn icon_button(icon: &[&'static str], tooltip: &str) -> gtk4::Button {
    let btn = gtk4::Button::from_icon_name(icons::pick(icon));
    btn.add_css_class("flat");
    btn.set_tooltip_text(Some(tooltip));
    btn.set_valign(gtk4::Align::Center);
    btn
}

/// Button content with an icon followed by a label.
pub fn icon_label(icon: &[&'static str], label: &str) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    b.append(&gtk4::Image::from_icon_name(icons::pick(icon)));
    b.append(&gtk4::Label::with_mnemonic(label));
    b
}

pub fn icon_label_button(icon: &[&'static str], label: &str) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.set_child(Some(&icon_label(icon, label)));
    btn
}

pub fn dropdown(items: &[&str]) -> gtk4::DropDown {
    let dd = gtk4::DropDown::from_strings(items);
    dd.set_hexpand(true);
    dd
}

pub fn entry(text: &str) -> gtk4::Entry {
    gtk4::Entry::builder().text(text).hexpand(true).build()
}

pub fn password_entry() -> gtk4::PasswordEntry {
    gtk4::PasswordEntry::builder().show_peek_icon(true).hexpand(true).build()
}

/// A titled block of "label : widget" rows laid out in a grid — the native
/// form pattern on KDE, XFCE and GNOME alike.
pub struct FormSection {
    pub root: gtk4::Box,
    grid: gtk4::Grid,
    rows: RefCell<i32>,
    labels: RefCell<Vec<gtk4::Label>>,
}

impl FormSection {
    pub fn new(title: Option<&str>, description: Option<&str>) -> Self {
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        if let Some(t) = title {
            root.append(&gtk4::Label::builder().label(t).xalign(0.0).css_classes(["form-section-title"]).build());
        }
        if let Some(d) = description {
            root.append(&gtk4::Label::builder().label(d).xalign(0.0).wrap(true).css_classes(["dim-label", "caption"]).build());
        }
        let grid = gtk4::Grid::builder().row_spacing(8).column_spacing(12).margin_top(4).build();
        root.append(&grid);
        Self { root, grid, rows: RefCell::new(0), labels: RefCell::new(Vec::new()) }
    }

    pub fn row(&self, label: &str, widget: &impl IsA<gtk4::Widget>) {
        let r = *self.rows.borrow();
        let lbl = gtk4::Label::builder().label(label).xalign(1.0).css_classes(["dim-label"]).build();
        lbl.set_mnemonic_widget(Some(widget));
        self.grid.attach(&lbl, 0, r, 1, 1);
        self.grid.attach(widget, 1, r, 1, 1);
        self.labels.borrow_mut().push(lbl);
        *self.rows.borrow_mut() = r + 1;
    }

    /// A full-width row (check boxes, button rows).
    pub fn wide(&self, widget: &impl IsA<gtk4::Widget>) {
        let r = *self.rows.borrow();
        self.grid.attach(widget, 1, r, 1, 1);
        *self.rows.borrow_mut() = r + 1;
    }
}

/// Check box with an optional dim hint line underneath.
pub fn check(label: &str, hint: Option<&str>, active: bool) -> (gtk4::Box, gtk4::CheckButton) {
    let b = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let cb = gtk4::CheckButton::builder().label(label).active(active).build();
    b.append(&cb);
    if let Some(h) = hint {
        b.append(&gtk4::Label::builder().label(h).xalign(0.0).wrap(true).margin_start(28).css_classes(["dim-label", "caption"]).build());
    }
    (b, cb)
}

/// In-window transient notification, styled with GTK's stock
/// `.app-notification` class (supported by Default/Adwaita and Breeze).
#[derive(Clone)]
pub struct Toaster {
    pub overlay: gtk4::Overlay,
    revealer: gtk4::Revealer,
    label: gtk4::Label,
    hide_src: Rc<RefCell<Option<glib::SourceId>>>,
}

impl Toaster {
    pub fn new() -> Self {
        let overlay = gtk4::Overlay::new();
        let label = gtk4::Label::builder().wrap(true).max_width_chars(60).build();
        let close = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::CLOSE));
        close.add_css_class("flat");
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        b.add_css_class("app-notification");
        b.append(&label);
        b.append(&close);
        let revealer = gtk4::Revealer::builder()
            .child(&b)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::End)
            .margin_bottom(48)
            .transition_type(gtk4::RevealerTransitionType::SlideUp)
            .build();
        overlay.add_overlay(&revealer);
        let r = revealer.clone();
        close.connect_clicked(move |_| r.set_reveal_child(false));
        Self { overlay, revealer, label, hide_src: Rc::new(RefCell::new(None)) }
    }

    pub fn show(&self, msg: &str) {
        self.label.set_text(msg);
        self.revealer.set_reveal_child(true);
        if let Some(id) = self.hide_src.borrow_mut().take() {
            id.remove();
        }
        let r = self.revealer.clone();
        let src = Rc::clone(&self.hide_src);
        let id = glib::timeout_add_local_once(std::time::Duration::from_secs(4), move || {
            r.set_reveal_child(false);
            src.borrow_mut().take();
        });
        *self.hide_src.borrow_mut() = Some(id);
    }
}

/// Yes/no confirmation using GTK's AlertDialog (themed by the desktop).
pub fn confirm<F: Fn() + 'static>(parent: &impl IsA<gtk4::Window>, heading: &str, body: &str, accept: &str, on_accept: F) {
    let dialog = gtk4::AlertDialog::builder()
        .modal(true)
        .message(heading)
        .detail(body)
        .buttons(["Vazgeç", accept])
        .cancel_button(0)
        .default_button(1)
        .build();
    dialog.choose(Some(parent), gio::Cancellable::NONE, move |res| {
        if res == Ok(1) {
            on_accept();
        }
    });
}

/// Error message with a "copy" action.
pub fn error<F: Fn() + 'static>(parent: &impl IsA<gtk4::Window>, body: &str, on_copy: F) {
    let dialog = gtk4::AlertDialog::builder()
        .modal(true)
        .message("Hata")
        .detail(body)
        .buttons(["Kopyala", "Tamam"])
        .cancel_button(1)
        .default_button(1)
        .build();
    dialog.choose(Some(parent), gio::Cancellable::NONE, move |res| {
        if res == Ok(0) {
            on_copy();
        }
    });
}

pub enum Field {
    Text(&'static str, String),
    Password(&'static str, String),
    /// Check box; its value is "1" or "0".
    Check(&'static str, bool),
}

/// Small modal form dialog. Calls `on_accept` with the field values in order.
pub fn form_dialog<F>(parent: Option<&gtk4::Window>, title: &str, body: Option<&str>, fields: Vec<Field>, accept: &str, on_accept: F)
where
    F: Fn(Vec<String>) + 'static,
{
    let win = gtk4::Window::builder()
        .title(title)
        .css_classes(["shift-dialog"])
        .modal(true)
        .resizable(false)
        .default_width(380)
        .build();
    if let Some(p) = parent {
        win.set_transient_for(Some(p));
    }

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(16).margin_bottom(16).margin_start(16).margin_end(16)
        .build();
    let form = FormSection::new(None, body);
    let mut getters: Vec<Box<dyn Fn() -> String>> = Vec::new();
    let mut first: Option<gtk4::Widget> = None;
    for f in fields {
        match f {
            Field::Text(label, value) => {
                let e = entry(&value);
                e.set_activates_default(true);
                form.row(label, &e);
                first.get_or_insert_with(|| e.clone().upcast());
                getters.push(Box::new(move || e.text().to_string()));
            }
            Field::Check(label, active) => {
                let (b, cb) = check(label, None, active);
                form.wide(&b);
                getters.push(Box::new(move || if cb.is_active() { "1".into() } else { "0".into() }));
            }
            Field::Password(label, value) => {
                let e = password_entry();
                e.set_text(&value);
                e.set_activates_default(true);
                form.row(label, &e);
                first.get_or_insert_with(|| e.clone().upcast());
                getters.push(Box::new(move || e.text().to_string()));
            }
        }
    }
    root.append(&form.root);

    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    buttons.set_halign(gtk4::Align::End);
    let cancel = gtk4::Button::with_mnemonic("_İptal");
    let ok = gtk4::Button::with_mnemonic(accept);
    ok.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&ok);
    root.append(&buttons);
    win.set_child(Some(&root));
    win.set_default_widget(Some(&ok));

    let w = win.clone();
    cancel.connect_clicked(move |_| w.close());
    let w = win.clone();
    ok.connect_clicked(move |_| {
        on_accept(getters.iter().map(|g| g()).collect());
        w.close();
    });

    let esc = gtk4::EventControllerKey::new();
    let w = win.clone();
    esc.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            w.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    win.add_controller(esc);

    win.present();
    if let Some(f) = first {
        f.grab_focus();
    }
}
