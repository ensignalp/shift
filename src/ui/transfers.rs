//! "Aktarımlar" page: uploads/downloads of this session and their state.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use gtk4::prelude::*;
use crate::ui::icons;

#[derive(Clone, Debug)]
pub enum TransferState {
    Running,
    Done,
    Failed(String),
}

struct Row {
    state: gtk4::Label,
    spinner: gtk4::Spinner,
}

pub struct TransfersPage {
    pub container: gtk4::Box,
    list: gtk4::ListBox,
    stack: gtk4::Stack,
    rows: RefCell<HashMap<u64, Row>>,
    /// Number of running transfers, shown on the sidebar nav item.
    pub active: Rc<RefCell<u32>>,
}

impl TransfersPage {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_top(24).margin_bottom(16).margin_start(28).margin_end(28)
            .build();
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.append(&gtk4::Label::builder().label("Aktarımlar").css_classes(["page-title"]).hexpand(true).xalign(0.0).build());
        let btn_clear = gtk4::Button::builder().label("Bitenleri temizle").css_classes(["soft"]).build();
        header.append(&btn_clear);
        container.append(&header);

        let stack = gtk4::Stack::builder().vexpand(true).build();
        let list = gtk4::ListBox::builder().selection_mode(gtk4::SelectionMode::None).css_classes(["card"]).valign(gtk4::Align::Start).build();
        list.set_overflow(gtk4::Overflow::Hidden);
        let scroll = gtk4::ScrolledWindow::builder().hscrollbar_policy(gtk4::PolicyType::Never).child(&list).build();
        stack.add_named(&scroll, Some("list"));

        let empty = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).spacing(10).valign(gtk4::Align::Center).halign(gtk4::Align::Center).build();
        let icon = gtk4::Image::from_icon_name(icons::pick(icons::TRANSFERS));
        icon.set_pixel_size(48);
        icon.add_css_class("dim");
        empty.append(&icon);
        empty.append(&gtk4::Label::builder().label("Aktarım yok").css_classes(["heading"]).build());
        empty.append(&gtk4::Label::builder().label("Yüklediğiniz ve indirdiğiniz dosyalar burada görünür.").css_classes(["dim"]).build());
        stack.add_named(&empty, Some("empty"));
        stack.set_visible_child_name("empty");
        container.append(&stack);

        let page = Self { container, list, stack, rows: RefCell::new(HashMap::new()), active: Rc::new(RefCell::new(0)) };

        let list = page.list.clone();
        let stack = page.stack.clone();
        btn_clear.connect_clicked(move |_| {
            let mut c = list.first_child();
            while let Some(row) = c {
                c = row.next_sibling();
                if row.widget_name() != "running" {
                    list.remove(&row);
                }
            }
            if list.first_child().is_none() {
                stack.set_visible_child_name("empty");
            }
        });
        page
    }

    pub fn update(&self, id: u64, name: &str, upload: bool, state: TransferState) {
        if !self.rows.borrow().contains_key(&id) {
            let b = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(12)
                .margin_top(10).margin_bottom(10).margin_start(14).margin_end(14).build();
            let tile = gtk4::Box::builder().css_classes(["host-tile"]).valign(gtk4::Align::Center).build();
            let img = gtk4::Image::from_icon_name(icons::pick(if upload { icons::UPLOAD } else { icons::DOWNLOAD }));
            img.set_hexpand(true);
            tile.set_hexpand(false);
            tile.append(&img);
            b.append(&tile);
            let text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
            text.set_hexpand(true);
            text.set_valign(gtk4::Align::Center);
            text.append(&gtk4::Label::builder().label(name).xalign(0.0).ellipsize(gtk4::pango::EllipsizeMode::Middle).css_classes(["heading"]).build());
            text.append(&gtk4::Label::builder().label(if upload { "Yükleme" } else { "İndirme" }).xalign(0.0).css_classes(["caption", "dim"]).build());
            b.append(&text);
            let spinner = gtk4::Spinner::new();
            b.append(&spinner);
            let state_lbl = gtk4::Label::builder().css_classes(["caption"]).build();
            b.append(&state_lbl);
            let row = gtk4::ListBoxRow::builder().child(&b).activatable(false).build();
            self.list.prepend(&row);
            self.rows.borrow_mut().insert(id, Row { state: state_lbl, spinner });
            self.stack.set_visible_child_name("list");
        }
        let rows = self.rows.borrow();
        let r = &rows[&id];
        let row_widget = r.state.ancestor(gtk4::ListBoxRow::static_type());
        for c in ["text-success", "text-danger", "dim"] {
            r.state.remove_css_class(c);
        }
        match state {
            TransferState::Running => {
                *self.active.borrow_mut() += 1;
                r.spinner.start();
                r.spinner.set_visible(true);
                r.state.set_label("Sürüyor…");
                r.state.add_css_class("dim");
                if let Some(w) = row_widget { w.set_widget_name("running"); }
            }
            TransferState::Done | TransferState::Failed(_) => {
                let mut a = self.active.borrow_mut();
                *a = a.saturating_sub(1);
                r.spinner.stop();
                r.spinner.set_visible(false);
                if let Some(w) = row_widget { w.set_widget_name("finished"); }
                if let TransferState::Failed(e) = state {
                    r.state.set_label("Hata");
                    r.state.set_tooltip_text(Some(&e));
                    r.state.add_css_class("text-danger");
                } else {
                    r.state.set_label("Tamamlandı");
                    r.state.add_css_class("text-success");
                }
            }
        }
    }
}
