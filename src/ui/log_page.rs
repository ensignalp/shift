//! "Günlük" page: protocol command/response log.

use gtk4::prelude::*;
use crate::ui::{icons, widgets};

pub struct LogPage {
    pub container: gtk4::Box,
    buffer: gtk4::TextBuffer,
    view: gtk4::TextView,
    /// Last log line, shown in the sidebar status card.
    pub last_line: gtk4::Label,
}

impl LogPage {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_top(24).margin_bottom(16).margin_start(28).margin_end(28)
            .build();

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.append(&gtk4::Label::builder().label("Günlük").css_classes(["page-title"]).hexpand(true).xalign(0.0).build());
        let btn_copy = widgets::icon_button(icons::COPY, "Günlüğü kopyala");
        header.append(&btn_copy);
        let btn_clear = widgets::icon_button(icons::CLEAR, "Temizle");
        header.append(&btn_clear);
        container.append(&header);

        let view = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(16).right_margin(16).top_margin(12).bottom_margin(12)
            .css_classes(["log-view"])
            .build();
        let buffer = view.buffer();
        let scroll = gtk4::ScrolledWindow::builder().vexpand(true).child(&view).build();
        let frame = gtk4::Frame::builder().child(&scroll).css_classes(["card", "files-card"]).build();
        frame.set_overflow(gtk4::Overflow::Hidden);
        container.append(&frame);

        let last_line = gtk4::Label::builder()
            .label("Hazır.")
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["caption", "dim"])
            .build();

        let b = buffer.clone();
        let l = last_line.clone();
        btn_clear.connect_clicked(move |_| {
            b.set_text("");
            l.set_text("Günlük temizlendi.");
        });
        let b = buffer.clone();
        btn_copy.connect_clicked(move |btn| {
            let (start, end) = b.bounds();
            btn.display().clipboard().set_text(&b.text(&start, &end, true));
        });

        Self { container, buffer, view, last_line }
    }

    pub fn append_log(&self, level: &str, msg: &str) {
        let prefix = match level {
            "command" => "Komut: ",
            "response" => "Yanıt:  ",
            "error" => "Hata:   ",
            "transfer" => "Aktarım: ",
            _ => "Durum:  ",
        };
        let time = chrono::Local::now().format("%H:%M:%S");
        let mut end = self.buffer.end_iter();
        self.buffer.insert(&mut end, &format!("{}  {}{}\n", time, prefix, msg));
        let mark = self.buffer.create_mark(None, &self.buffer.end_iter(), false);
        self.view.scroll_mark_onscreen(&mark);
        self.buffer.delete_mark(&mark);
        self.last_line.set_text(msg);
    }
}
