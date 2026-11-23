#![allow(dead_code)]
use gtk4::prelude::*;

pub struct LogRevealer {
    pub container: gtk4::Box,
    pub revealer: gtk4::Revealer,
    pub buffer: gtk4::TextBuffer,
    pub toggle_btn: gtk4::ToggleButton,
    pub status_label: gtk4::Label,
}

impl LogRevealer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .reveal_child(false)
            .build();

        let panel_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .height_request(180)
            .css_classes(["log-panel-container"])
            .build();

        // Control Toolbar
        let ctrl_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .margin_start(16)
            .margin_end(16)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        let title = gtk4::Label::builder()
            .label("Komut ve İşlem Günlüğü")
            .css_classes(["caption", "bold"])
            .build();
        ctrl_bar.append(&title);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        ctrl_bar.append(&spacer);

        let btn_copy = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::COPY));
        btn_copy.set_tooltip_text(Some("Günlüğü Kopyala"));
        btn_copy.add_css_class("flat");
        btn_copy.add_css_class("circular");
        ctrl_bar.append(&btn_copy);

        let btn_clear = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::DELETE));
        btn_clear.set_tooltip_text(Some("Temizle"));
        btn_clear.add_css_class("flat");
        btn_clear.add_css_class("circular");
        ctrl_bar.append(&btn_clear);

        let btn_close = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::COLLAPSE));
        btn_close.set_tooltip_text(Some("Konsolu Gizle"));
        btn_close.add_css_class("flat");
        btn_close.add_css_class("circular");
        ctrl_bar.append(&btn_close);

        panel_box.append(&ctrl_bar);

        let text_view = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(16)
            .right_margin(16)
            .top_margin(8)
            .bottom_margin(8)
            .css_classes(["log-console-view"])
            .build();

        let buffer = text_view.buffer();
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&text_view)
            .build();
        panel_box.append(&scroll);

        revealer.set_child(Some(&panel_box));
        container.append(&revealer);

        // Bottom status line with terminal icon
        let status_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_start(16)
            .margin_end(16)
            .margin_top(4)
            .margin_bottom(6)
            .build();

        let status_label = gtk4::Label::builder()
            .label("Hazır.")
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["caption", "dim-label"])
            .build();
        status_bar.append(&status_label);

        let toggle_btn = gtk4::ToggleButton::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::TERMINAL))
            .tooltip_text("Günlük Konsolunu Göster/Gizle")
            .css_classes(["flat", "circular"])
            .build();
        status_bar.append(&toggle_btn);

        container.append(&status_bar);

        // Events
        let r_clone = revealer.clone();
        toggle_btn.connect_toggled(move |btn| {
            r_clone.set_reveal_child(btn.is_active());
        });

        let t_clone = toggle_btn.clone();
        btn_close.connect_clicked(move |_| {
            t_clone.set_active(false);
        });

        let b_clone = buffer.clone();
        let s_clone = status_label.clone();
        btn_clear.connect_clicked(move |_| {
            b_clone.set_text("");
            s_clone.set_text("Konsol temizlendi.");
        });

        let b_clone2 = buffer.clone();
        btn_copy.connect_clicked(move |btn| {
            let (start, end) = b_clone2.bounds();
            let text = b_clone2.text(&start, &end, true);
            let display = btn.display();
            display.clipboard().set_text(&text);
        });

        Self {
            container,
            revealer,
            buffer,
            toggle_btn,
            status_label,
        }
    }

    pub fn append_log(&self, level: &str, msg: &str) {
        let prefix = match level {
            "command" => "Komut: ",
            "response" => "Yanıt:  ",
            "error" => "Hata:   ",
            "transfer" => "Aktarım: ",
            _ => "Durum:  ",
        };
        let line = format!("{}{}\n", prefix, msg);
        let mut end = self.buffer.end_iter();
        self.buffer.insert(&mut end, &line);

        let preview = if msg.len() < 80 { msg } else { &msg[..77] };
        self.status_label.set_text(&format!("{}{}", prefix, preview));
    }
}
