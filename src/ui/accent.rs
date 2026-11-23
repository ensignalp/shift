//! Desktop accent colour from the XDG settings portal
//! (org.freedesktop.appearance accent-color), via plain GIO D-Bus.
//! GNOME 47+, KDE Plasma 6 and others publish it; when absent the callback
//! receives `None` and Shift falls back to its brand blue.

use std::rc::Rc;
use glib::prelude::*;
use crate::ui::theme::Rgb;

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_IFACE: &str = "org.freedesktop.portal.Settings";

fn parse_rgb(v: &glib::Variant) -> Option<Rgb> {
    // The value may arrive wrapped in one or more "v" layers.
    let mut v = v.clone();
    while v.type_() == glib::VariantTy::VARIANT {
        match v.as_variant() {
            Some(inner) => v = inner,
            None => break,
        }
    }
    let (r, g, b) = v.get::<Rgb>()?;
    let ok = |c: f64| (0.0..=1.0).contains(&c);
    (ok(r) && ok(g) && ok(b)).then_some((r, g, b))
}

pub fn watch<F: Fn(Option<Rgb>) + 'static>(on_change: F) {
    let on_change: Rc<dyn Fn(Option<Rgb>)> = Rc::new(on_change);
    gio::bus_get(gio::BusType::Session, gio::Cancellable::NONE, move |res| {
        let Ok(conn) = res else { return };

        let cb = Rc::clone(&on_change);
        #[allow(deprecated)]
        let _id = conn.signal_subscribe(
            Some(PORTAL),
            Some(SETTINGS_IFACE),
            Some("SettingChanged"),
            Some(PORTAL_PATH),
            Some("org.freedesktop.appearance"),
            gio::DBusSignalFlags::NONE,
            move |_, _, _, _, _, params| {
                if params.n_children() == 3 && params.child_value(1).str() == Some("accent-color") {
                    cb(parse_rgb(&params.child_value(2)));
                }
            },
        );

        let cb = Rc::clone(&on_change);
        conn.call(
            Some(PORTAL),
            PORTAL_PATH,
            SETTINGS_IFACE,
            "ReadOne",
            Some(&("org.freedesktop.appearance", "accent-color").to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            1000,
            gio::Cancellable::NONE,
            move |reply| {
                if let Ok(v) = reply {
                    cb(parse_rgb(&v.child_value(0)));
                }
            },
        );
    });
}
