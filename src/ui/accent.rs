//! Desktop accent colour for GTK's built-in theme.
//!
//! Themes such as Breeze, Yaru or Greybird already carry the desktop's
//! accent/selection colour. GTK's built-in "Default"/"Adwaita" theme has a
//! fixed blue, and libadwaita (which we no longer use) used to recolour it
//! from the XDG settings portal. This module does the same, with plain GIO
//! D-Bus calls, and only while the built-in theme is active.

use std::rc::Rc;
use std::cell::Cell;
use gtk4::prelude::*;

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_IFACE: &str = "org.freedesktop.portal.Settings";

type Rgb = (f64, f64, f64);

fn is_builtin_theme(settings: &gtk4::Settings) -> bool {
    let name = settings.gtk_theme_name().map(|n| n.to_lowercase()).unwrap_or_default();
    name.is_empty() || name == "default" || name.starts_with("adwaita")
}

fn parse_rgb(v: &glib::Variant) -> Option<Rgb> {
    // Value may arrive wrapped in one or two "v" layers depending on the call.
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

fn css_for((r, g, b): Rgb) -> String {
    let c = format!("rgb({},{},{})", (r * 255.0).round(), (g * 255.0).round(), (b * 255.0).round());
    format!(
        "@define-color accent_color {c};
@define-color accent_bg_color {c};
@define-color theme_selected_bg_color {c};
button.suggested-action {{ background-image: none; background-color: {c}; border-color: shade({c}, 0.8); color: white; }}
button.suggested-action:hover {{ background-color: shade({c}, 1.08); }}
button.suggested-action:active, button.suggested-action:checked {{ background-color: shade({c}, 0.88); }}
check:checked, radio:checked, check:indeterminate, radio:indeterminate {{ background-image: none; background-color: {c}; border-color: shade({c}, 0.8); color: white; }}
switch:checked {{ background-color: {c}; border-color: shade({c}, 0.8); }}
treeview.view:selected, row:selected, columnview row:selected, flowboxchild:selected {{ background-color: {c}; color: white; }}
selection, entry selection, text selection, label selection {{ background-color: alpha({c}, 0.6); color: white; }}
entry:focus-within, spinbutton:focus-within, button:focus-visible, dropdown > button:focus-visible {{ outline-color: alpha({c}, 0.6); }}
progressbar progress, scale highlight, levelbar block.filled {{ background-color: {c}; border-color: {c}; }}
link, button.link {{ color: {c}; }}
"
    )
}

pub fn install(display: &gtk4::gdk::Display) {
    let Some(settings) = gtk4::Settings::default() else { return };
    let provider = gtk4::CssProvider::new();
    // Above theme + settings providers, below application and user (~/.config/gtk-4.0/gtk.css) CSS.
    gtk4::style_context_add_provider_for_display(display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_SETTINGS + 50);

    if std::env::var("SHIFT_DEBUG").is_ok() {
        provider.connect_parsing_error(|_, section, err| eprintln!("accent css error: {} at {}", err, section));
    }
    let current: Rc<Cell<Option<Rgb>>> = Rc::new(Cell::new(None));
    let apply: Rc<dyn Fn()> = {
        let provider = provider.clone();
        let settings = settings.clone();
        let current = Rc::clone(&current);
        Rc::new(move || match current.get() {
            Some(rgb) if is_builtin_theme(&settings) => provider.load_from_string(&css_for(rgb)),
            _ => provider.load_from_string(""),
        })
    };

    // Re-evaluate when the theme changes (e.g. user switches to Breeze).
    let apply_theme = Rc::clone(&apply);
    settings.connect_gtk_theme_name_notify(move |_| apply_theme());

    gio::bus_get(gio::BusType::Session, gio::Cancellable::NONE, move |res| {
        let Ok(conn) = res else { return };

        // Live updates.
        let cur_sig = Rc::clone(&current);
        let apply_sig = Rc::clone(&apply);
        #[allow(deprecated)]
        let _id = conn.signal_subscribe(
            Some(PORTAL),
            Some(SETTINGS_IFACE),
            Some("SettingChanged"),
            Some(PORTAL_PATH),
            Some("org.freedesktop.appearance"),
            gio::DBusSignalFlags::NONE,
            move |_, _, _, _, _, params| {
                if params.n_children() == 3
                    && params.child_value(1).str() == Some("accent-color")
                {
                    cur_sig.set(parse_rgb(&params.child_value(2)));
                    apply_sig();
                }
            },
        );

        // Initial value.
        let cur_init = Rc::clone(&current);
        let apply_init = Rc::clone(&apply);
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
                    cur_init.set(parse_rgb(&v.child_value(0)));
                    apply_init();
                }
            },
        );
    });
}
