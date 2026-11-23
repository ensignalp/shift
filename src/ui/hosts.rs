//! "Sunucular" page — Termius-style host vault:
//! quick-connect omnibox, host cards, slide-in editor panel.

use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;

use crate::models::{Protocol, ServerProfile};
use crate::server_store::ServerStore;
use crate::ui::icons;
use crate::ui::widgets::{self, Field};

pub const PROTOCOLS: &[&str] = &["SFTP", "FTP", "FTPS"];

pub fn proto_from_index(i: u32) -> Protocol {
    match i {
        1 => Protocol::Ftp,
        2 => Protocol::Ftps,
        _ => Protocol::Sftp,
    }
}

fn proto_index(p: Protocol) -> u32 {
    match p {
        Protocol::Sftp => 0,
        Protocol::Ftp => 1,
        Protocol::Ftps => 2,
    }
}

fn default_port(p: Protocol) -> u16 {
    if p == Protocol::Sftp { 22 } else { 21 }
}

/// Parse "[proto://][user@]host[:port][/path]".
fn parse_target(input: &str, fallback: Protocol) -> Option<(Protocol, String, String, u16, String)> {
    let mut rest = input.trim();
    if rest.is_empty() {
        return None;
    }
    let mut proto = fallback;
    if let Some((scheme, r)) = rest.split_once("://") {
        proto = match scheme.to_lowercase().as_str() {
            "sftp" | "ssh" => Protocol::Sftp,
            "ftp" => Protocol::Ftp,
            "ftps" => Protocol::Ftps,
            _ => return None,
        };
        rest = r;
    }
    let (rest, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    let (user, hostport) = match rest.rsplit_once('@') {
        Some((u, h)) => (u.to_string(), h),
        None => (String::new(), rest),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()?),
        None => (hostport.to_string(), default_port(proto)),
    };
    if host.is_empty() {
        return None;
    }
    Some((proto, user, host, port, path))
}

/// Label above a widget (Termius / quick-send form style).
fn field(label: &str, w: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    b.append(&gtk4::Label::builder().label(label).xalign(0.0).css_classes(["caption", "dim"]).build());
    b.append(w);
    b
}

fn card(children: &[&gtk4::Widget]) -> gtk4::Box {
    let c = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .css_classes(["card", "padded"])
        .build();
    for ch in children {
        c.append(*ch);
    }
    c
}

fn section(title: &str) -> gtk4::Label {
    gtk4::Label::builder().label(title).xalign(0.0).css_classes(["section-title"]).margin_top(4).build()
}

struct Editor {
    revealer: gtk4::Revealer,
    title: gtk4::Label,
    editing: RefCell<Option<ServerProfile>>,
    name: gtk4::Entry,
    proto: gtk4::DropDown,
    host: gtk4::Entry,
    port: gtk4::Entry,
    user: gtk4::Entry,
    password: gtk4::PasswordEntry,
    key: gtk4::Entry,
    def_local: gtk4::Entry,
    def_remote: gtk4::Entry,
    btn_delete: gtk4::Button,
}

impl Editor {
    fn open(&self, p: Option<&ServerProfile>) {
        *self.editing.borrow_mut() = p.cloned();
        self.host.remove_css_class("error");
        match p {
            Some(p) => {
                self.title.set_label("Sunucuyu düzenle");
                self.name.set_text(&p.name);
                self.proto.set_selected(proto_index(p.proto));
                self.port.set_text(&p.port.to_string());
                self.host.set_text(&p.host);
                self.user.set_text(&p.user);
                self.password.set_text(p.password.as_deref().unwrap_or(""));
                self.def_local.set_text(p.default_local_dir.as_deref().unwrap_or(""));
                self.def_remote.set_text(p.default_remote_dir.as_deref().unwrap_or("/"));
                self.btn_delete.set_visible(true);
            }
            None => {
                self.title.set_label("Yeni sunucu");
                self.name.set_text("");
                self.proto.set_selected(0);
                self.port.set_text("22");
                self.host.set_text("");
                self.user.set_text("");
                self.password.set_text("");
                self.def_local.set_text("~/Projeler");
                self.def_remote.set_text("/var/www/html");
                self.btn_delete.set_visible(false);
            }
        }
        self.revealer.set_reveal_child(true);
        if p.is_some() { self.name.grab_focus(); } else { self.host.grab_focus(); }
    }

    fn close(&self) {
        self.revealer.set_reveal_child(false);
    }

    /// Profile from the form; None (and the field flagged) if the host is empty.
    fn profile(&self) -> Option<ServerProfile> {
        let host = self.host.text().trim().to_string();
        if host.is_empty() {
            self.host.add_css_class("error");
            self.host.grab_focus();
            return None;
        }
        self.host.remove_css_class("error");
        let user = self.user.text().trim().to_string();
        let name = self.name.text().trim().to_string();
        let proto = proto_from_index(self.proto.selected());
        let id = self
            .editing
            .borrow()
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| format!("{}_{}", host, user));
        Some(ServerProfile {
            id,
            name: if name.is_empty() { host.clone() } else { name },
            port: self.port.text().trim().parse().unwrap_or(default_port(proto)),
            host,
            user,
            password: Some(self.password.text().to_string()),
            proto,
            default_remote_dir: Some(self.def_remote.text().trim().to_string()),
            default_local_dir: Some(self.def_local.text().trim().to_string()),
        })
    }
}

pub struct HostsPage {
    pub container: gtk4::Box,
    pub reload: Rc<dyn Fn()>,
    pub new_host: Rc<dyn Fn()>,
}

impl HostsPage {
    pub fn new<F>(on_connect: F) -> Self
    where
        F: Fn(ServerProfile) + 'static + Clone,
    {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

        // ------------------------------------------------------------ main
        let main = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .hexpand(true)
            .margin_top(24).margin_bottom(16).margin_start(28).margin_end(28)
            .build();

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        header.append(&gtk4::Label::builder().label("Sunucular").css_classes(["page-title"]).build());
        let count_chip = gtk4::Label::builder().label("0").css_classes(["chip"]).valign(gtk4::Align::Center).build();
        header.append(&count_chip);
        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
        let btn_new = widgets::icon_label_button(icons::ADD, "Yeni sunucu");
        btn_new.add_css_class("accent");
        header.append(&btn_new);
        main.append(&header);

        // Quick connect omnibox
        let omni_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let omnibox = gtk4::Entry::builder()
            .placeholder_text("Hızlı bağlan: kullanıcı@adres:port   (ör. sftp://root@192.168.1.10)")
            .primary_icon_name(icons::pick(icons::QUICK))
            .hexpand(true)
            .css_classes(["omnibox"])
            .build();
        omni_row.append(&omnibox);
        let omni_proto = widgets::dropdown(PROTOCOLS);
        omni_proto.set_hexpand(false);
        omni_proto.set_tooltip_text(Some("Adreste protokol yazılmazsa kullanılır"));
        omni_row.append(&omni_proto);
        let btn_quick = gtk4::Button::builder().label("Bağlan").css_classes(["soft"]).build();
        omni_row.append(&btn_quick);
        main.append(&omni_row);

        // Host grid / empty state
        let list_stack = gtk4::Stack::builder().vexpand(true).build();

        let grid_box = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        grid_box.append(&section("TÜM SUNUCULAR"));
        let flow = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .activate_on_single_click(true)
            .homogeneous(true)
            .min_children_per_line(1)
            .max_children_per_line(4)
            .column_spacing(12)
            .row_spacing(12)
            .valign(gtk4::Align::Start)
            .css_classes(["host-grid"])
            .build();
        let scroller = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&flow)
            .build();
        grid_box.append(&scroller);
        list_stack.add_named(&grid_box, Some("grid"));

        let empty = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .build();
        let empty_icon = gtk4::Image::from_icon_name("com.ensignsoft.Shift");
        empty_icon.set_pixel_size(96);
        empty.append(&empty_icon);
        empty.append(&gtk4::Label::builder().label("Henüz kayıtlı sunucu yok").css_classes(["heading"]).build());
        empty.append(&gtk4::Label::builder()
            .label("Yukarıdaki çubuktan hızlıca bağlanın veya bir sunucu ekleyin.")
            .css_classes(["dim"])
            .build());
        let btn_empty_new = gtk4::Button::builder().label("Sunucu ekle").css_classes(["accent"]).halign(gtk4::Align::Center).margin_top(6).build();
        empty.append(&btn_empty_new);
        list_stack.add_named(&empty, Some("empty"));
        main.append(&list_stack);
        container.append(&main);

        // ---------------------------------------------------------- editor
        let editor = Rc::new(Self::build_editor());
        container.append(&editor.revealer);

        // ---------------------------------------------------------- wiring
        let rows: Rc<RefCell<Vec<ServerProfile>>> = Rc::new(RefCell::new(Vec::new()));
        let reload: Rc<dyn Fn()> = {
            let flow = flow.clone();
            let rows = Rc::clone(&rows);
            let list_stack = list_stack.clone();
            let count_chip = count_chip.clone();
            let editor = Rc::clone(&editor);
            Rc::new(move || {
                while let Some(child) = flow.first_child() {
                    flow.remove(&child);
                }
                let all = ServerStore::new().get_all();
                count_chip.set_label(&all.len().to_string());
                list_stack.set_visible_child_name(if all.is_empty() { "empty" } else { "grid" });
                for srv in &all {
                    flow.append(&Self::host_card(srv, &editor));
                }
                *rows.borrow_mut() = all;
            })
        };

        let on_c = on_connect.clone();
        let rows_a = Rc::clone(&rows);
        flow.connect_child_activated(move |_, child| {
            let srv = rows_a.borrow().get(child.index() as usize).cloned();
            if let Some(srv) = srv {
                on_c(srv);
            }
        });

        let new_host: Rc<dyn Fn()> = {
            let editor = Rc::clone(&editor);
            Rc::new(move || editor.open(None))
        };
        let nh = Rc::clone(&new_host);
        btn_new.connect_clicked(move |_| nh());
        let nh = Rc::clone(&new_host);
        btn_empty_new.connect_clicked(move |_| nh());

        // Editor port follows protocol (only when it is still a default port)
        let port_auto = editor.port.clone();
        editor.proto.connect_selected_notify(move |dd| {
            let cur = port_auto.text();
            if cur.is_empty() || cur == "21" || cur == "22" {
                port_auto.set_text(&default_port(proto_from_index(dd.selected())).to_string());
            }
        });

        // Editor buttons
        let ed = Rc::clone(&editor);
        let rl = Rc::clone(&reload);
        let save = move || -> Option<ServerProfile> {
            let p = ed.profile()?;
            let store = ServerStore::new();
            if ed.editing.borrow().is_some() {
                store.update_server(p.clone());
            } else {
                store.add_server(p.clone());
            }
            *ed.editing.borrow_mut() = Some(p.clone());
            rl();
            Some(p)
        };
        let save = Rc::new(save);

        let (sv, ed) = (Rc::clone(&save), Rc::clone(&editor));
        editor_button(&editor.revealer, "save").connect_clicked(move |_| {
            if sv().is_some() {
                ed.close();
            }
        });

        let (ed, on_c) = (Rc::clone(&editor), on_connect.clone());
        editor_button(&editor.revealer, "connect").connect_clicked(move |_| {
            if let Some(p) = ed.profile() {
                ed.close();
                on_c(p);
            }
        });

        let ed = Rc::clone(&editor);
        editor_button(&editor.revealer, "close").connect_clicked(move |_| ed.close());

        let (ed, rl) = (Rc::clone(&editor), Rc::clone(&reload));
        editor.btn_delete.connect_clicked(move |btn| {
            let Some(p) = ed.editing.borrow().clone() else { return };
            let Some(win) = btn.root().and_downcast::<gtk4::Window>() else { return };
            let (ed2, rl2) = (Rc::clone(&ed), Rc::clone(&rl));
            widgets::confirm(&win, "Sunucu silinsin mi?", &format!("'{}' kayıtlı sunucularınızdan silinecek.", p.name), "Sil", move || {
                ServerStore::new().delete_server(&p.id);
                ed2.close();
                rl2();
            });
        });

        // SSH key picker
        let key = editor.key.clone();
        editor_button(&editor.revealer, "browse-key").connect_clicked(move |btn| {
            let dialog = gtk4::FileDialog::builder().title("SSH özel anahtarını seç").modal(true).build();
            let key = key.clone();
            let parent = btn.root().and_downcast::<gtk4::Window>();
            dialog.open(parent.as_ref(), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        key.set_text(&p.display().to_string());
                    }
                }
            });
        });

        // Quick connect
        let quick = {
            let omnibox = omnibox.clone();
            let omni_proto = omni_proto.clone();
            let on_c = on_connect.clone();
            let rl = Rc::clone(&reload);
            move || {
                let Some((proto, user, host, port, path)) = parse_target(&omnibox.text(), proto_from_index(omni_proto.selected())) else {
                    omnibox.add_css_class("error");
                    omnibox.grab_focus();
                    return;
                };
                omnibox.remove_css_class("error");
                let user = if user.is_empty() { "anonymous".to_string() } else { user };
                let parent = omnibox.root().and_downcast::<gtk4::Window>();
                let (on_c, rl, omnibox) = (on_c.clone(), Rc::clone(&rl), omnibox.clone());
                let title = format!("{}@{}", user, host);
                widgets::form_dialog(
                    parent.as_ref(),
                    &title,
                    Some(&format!("{} · port {}", proto.as_str().to_uppercase(), port)),
                    vec![Field::Password("Parola", String::new()), Field::Check("Kayıtlı sunuculara ekle", true)],
                    "_Bağlan",
                    move |v| {
                        let profile = ServerProfile {
                            id: format!("{}_{}", host, user),
                            name: host.clone(),
                            host: host.clone(),
                            port,
                            user: user.clone(),
                            password: Some(v[0].clone()),
                            proto,
                            default_remote_dir: Some(path.clone()),
                            default_local_dir: None,
                        };
                        if v[1] == "1" {
                            ServerStore::new().add_server(profile.clone());
                            rl();
                        }
                        omnibox.set_text("");
                        on_c(profile);
                    },
                );
            }
        };
        let quick = Rc::new(quick);
        let q = Rc::clone(&quick);
        omnibox.connect_activate(move |_| q());
        let q = Rc::clone(&quick);
        btn_quick.connect_clicked(move |_| q());

        reload();
        Self { container, reload, new_host }
    }

    fn host_card(srv: &ServerProfile, editor: &Rc<Editor>) -> gtk4::Box {
        let card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .css_classes(["card", "host-card"])
            .build();
        let top = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let tile = gtk4::Box::builder().css_classes(["host-tile"]).valign(gtk4::Align::Center).build();
        match srv.proto {
            Protocol::Ftp => tile.add_css_class("ftp"),
            Protocol::Ftps => tile.add_css_class("ftps"),
            Protocol::Sftp => {}
        }
        let tile_icon = gtk4::Image::from_icon_name(icons::pick(icons::SERVER));
        tile_icon.set_halign(gtk4::Align::Center);
        tile_icon.set_valign(gtk4::Align::Center);
        tile_icon.set_hexpand(true);
        tile.set_hexpand(false);
        tile.set_halign(gtk4::Align::Start);
        tile.append(&tile_icon);
        top.append(&tile);

        let text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text.set_hexpand(true);
        text.set_valign(gtk4::Align::Center);
        text.append(&gtk4::Label::builder().label(&srv.name).xalign(0.0).ellipsize(gtk4::pango::EllipsizeMode::End).css_classes(["heading"]).build());
        text.append(&gtk4::Label::builder()
            .label(&format!("{}@{}", srv.user, srv.host))
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .css_classes(["caption", "dim"])
            .build());
        top.append(&text);

        let btn_edit = widgets::icon_button(icons::RENAME, "Düzenle");
        btn_edit.add_css_class("flat");
        let (ed, p) = (Rc::clone(editor), srv.clone());
        btn_edit.connect_clicked(move |_| ed.open(Some(&p)));
        top.append(&btn_edit);
        card.append(&top);

        let chips = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        chips.append(&gtk4::Label::builder().label(&srv.proto.as_str().to_uppercase()).css_classes(["chip"]).build());
        chips.append(&gtk4::Label::builder().label(&format!("port {}", srv.port)).css_classes(["chip"]).build());
        card.append(&chips);
        card
    }

    fn build_editor() -> Editor {
        let panel = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .width_request(380)
            .css_classes(["editor-panel"])
            .build();

        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .margin_top(16).margin_bottom(8).margin_start(20).margin_end(12)
            .build();
        let title = gtk4::Label::builder().label("Yeni sunucu").xalign(0.0).hexpand(true).css_classes(["page-title"]).build();
        head.append(&title);
        let btn_close = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::CLOSE));
        btn_close.add_css_class("flat");
        btn_close.set_tooltip_text(Some("Kapat"));
        btn_close.set_widget_name("close");
        head.append(&btn_close);
        panel.append(&head);

        let body = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(20).margin_end(20).margin_bottom(16)
            .build();

        // Address
        let name = widgets::entry("");
        name.set_placeholder_text(Some("ör. Üretim sunucusu"));
        let proto = widgets::dropdown(PROTOCOLS);
        let port = widgets::entry("22");
        port.set_input_purpose(gtk4::InputPurpose::Digits);
        port.set_width_chars(6);
        port.set_hexpand(false);
        let host = widgets::entry("");
        host.set_placeholder_text(Some("ornek.com veya 192.168.1.10"));
        let proto_port = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        let fp = field("Protokol", &proto);
        fp.set_hexpand(true);
        proto_port.append(&fp);
        proto_port.append(&field("Port", &port));
        body.append(&section("ADRES"));
        body.append(&card(&[
            field("Etiket", &name).upcast_ref(),
            field("Adres", &host).upcast_ref(),
            proto_port.upcast_ref(),
        ]));

        // Credentials
        let user = widgets::entry("");
        let password = widgets::password_entry();
        let key_box = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(6).build();
        let key = widgets::entry("");
        key.set_placeholder_text(Some("~/.ssh/id_ed25519"));
        key_box.append(&key);
        let btn_key = gtk4::Button::from_icon_name(icons::pick(icons::FOLDER_OPEN));
        btn_key.add_css_class("soft");
        btn_key.set_tooltip_text(Some("Anahtar dosyası seç"));
        btn_key.set_widget_name("browse-key");
        key_box.append(&btn_key);
        body.append(&section("KİMLİK"));
        body.append(&card(&[
            field("Kullanıcı adı", &user).upcast_ref(),
            field("Parola", &password).upcast_ref(),
            field("SSH anahtarı", &key_box).upcast_ref(),
        ]));

        // Advanced (former site manager tabs)
        let def_local = widgets::entry("~/Projeler");
        let def_remote = widgets::entry("/var/www/html");
        let (sync_box, _) = widgets::check("Eşzamanlı gezinme", Some("Yerel ve uzak tarafta aynı anda dizin değiştir"), false);
        let (hidden_box, _) = widgets::check("Gizli dosyaları her zaman göster", None, false);
        let adv = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).spacing(12).margin_top(10).build();
        adv.append(&card(&[
            field("Varsayılan yerel dizin", &def_local).upcast_ref(),
            field("Varsayılan uzak dizin", &def_remote).upcast_ref(),
            sync_box.upcast_ref(),
            hidden_box.upcast_ref(),
        ]));
        adv.append(&card(&[
            field("Aktarım modu", &widgets::dropdown(&["Pasif (önerilen)", "Aktif", "Varsayılan"])).upcast_ref(),
            field("İndirme sınırı", &widgets::dropdown(&["Sınırsız", "500 KB/s", "1 MB/s", "5 MB/s"])).upcast_ref(),
            field("Yükleme sınırı", &widgets::dropdown(&["Sınırsız", "500 KB/s", "1 MB/s", "5 MB/s"])).upcast_ref(),
            field("Dosya zaten varsa", &widgets::dropdown(&["Üzerine yaz", "Atla", "Yeniden adlandır", "Sor"])).upcast_ref(),
        ]));
        adv.append(&card(&[
            field("Karakter kodlaması", &widgets::dropdown(&["Otomatik / UTF-8", "ISO-8859-9 (Türkçe Latin-5)", "Windows-1254 (Türkçe)"])).upcast_ref(),
            field("Proxy", &widgets::dropdown(&["Yok", "SOCKS5", "HTTP/1.1", "FTP proxy"])).upcast_ref(),
        ]));
        let expander = gtk4::Expander::builder().label("GELİŞMİŞ").child(&adv).build();
        body.append(&expander);

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&body)
            .build();
        panel.append(&scroll);

        let foot = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(12).margin_bottom(16).margin_start(20).margin_end(20)
            .build();
        let btn_delete = gtk4::Button::builder().label("Sil").css_classes(["danger"]).visible(false).build();
        foot.append(&btn_delete);
        let sp = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        foot.append(&sp);
        let btn_save = gtk4::Button::builder().label("Kaydet").css_classes(["soft"]).build();
        btn_save.set_widget_name("save");
        foot.append(&btn_save);
        let btn_connect = gtk4::Button::builder().label("Bağlan").css_classes(["accent"]).build();
        btn_connect.set_widget_name("connect");
        foot.append(&btn_connect);
        panel.append(&foot);

        panel.set_hexpand(false);
        let revealer = gtk4::Revealer::builder()
            .hexpand(false)
            .transition_type(gtk4::RevealerTransitionType::SlideLeft)
            .reveal_child(false)
            .child(&panel)
            .build();

        Editor {
            revealer,
            title,
            editing: RefCell::new(None),
            name,
            proto,
            host,
            port,
            user,
            password,
            key,
            def_local,
            def_remote,
            btn_delete,
        }
    }
}

/// Find a named button inside the editor panel.
fn editor_button(root: &impl IsA<gtk4::Widget>, name: &str) -> gtk4::Button {
    fn find(w: &gtk4::Widget, name: &str) -> Option<gtk4::Button> {
        if w.widget_name() == name {
            if let Ok(b) = w.clone().downcast::<gtk4::Button>() {
                return Some(b);
            }
        }
        let mut c = w.first_child();
        while let Some(ch) = c {
            if let Some(b) = find(&ch, name) {
                return Some(b);
            }
            c = ch.next_sibling();
        }
        None
    }
    find(root.upcast_ref(), name).expect("editor button")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quick_connect_targets() {
        let (p, u, h, port, path) = parse_target("sftp://root@10.0.0.1:2222/var", Protocol::Ftp).unwrap();
        assert_eq!((p, u.as_str(), h.as_str(), port, path.as_str()), (Protocol::Sftp, "root", "10.0.0.1", 2222, "/var"));
        let (p, u, h, port, _) = parse_target("ftp.example.com", Protocol::Ftp).unwrap();
        assert_eq!((p, u.as_str(), h.as_str(), port), (Protocol::Ftp, "", "ftp.example.com", 21));
        let (p, _, _, port, _) = parse_target("deploy@web", Protocol::Sftp).unwrap();
        assert_eq!((p, port), (Protocol::Sftp, 22));
        assert!(parse_target("  ", Protocol::Sftp).is_none());
        assert!(parse_target("gopher://x", Protocol::Sftp).is_none());
    }
}
