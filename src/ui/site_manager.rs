use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use crate::models::{Protocol, ServerProfile};
use crate::server_store::ServerStore;
use crate::ui::icons;
use crate::ui::onboarding::{proto_from_index, PROTOCOLS};
use crate::ui::widgets::{self, FormSection};

pub struct SiteManagerWindow;

fn page(sections: &[&FormSection]) -> gtk4::ScrolledWindow {
    let b = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(24)
        .margin_top(16).margin_bottom(16).margin_start(16).margin_end(16)
        .build();
    for s in sections {
        b.append(&s.root);
    }
    gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&b)
        .build()
}

impl SiteManagerWindow {
    pub fn show<F>(parent: &gtk4::Window, on_connect: F)
    where
        F: Fn(ServerProfile) + 'static + Clone,
    {
        let window = gtk4::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Site Yöneticisi")
            .default_width(560)
            .default_height(600)
            .build();

        let store = Rc::new(RefCell::new(ServerStore::new()));
        let profiles = Rc::new(RefCell::new(store.borrow().get_all()));

        let main_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Profile picker: [saved sites ▾] [save] [delete]
        let picker = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(12).margin_bottom(12).margin_start(16).margin_end(16)
            .build();
        picker.append(&gtk4::Label::builder().label("Site").css_classes(["dim-label"]).build());
        let site_combo = widgets::dropdown(&["(Yeni profil)"]);
        picker.append(&site_combo);
        let btn_save = widgets::icon_button(icons::SAVE, "Bu profili kaydet / güncelle");
        picker.append(&btn_save);
        let btn_del = widgets::icon_button(icons::DELETE, "Bu profili sil");
        picker.append(&btn_del);
        main_vbox.append(&picker);

        let update_combo_model = {
            let site_combo = site_combo.clone();
            let profiles = Rc::clone(&profiles);
            move || {
                let mut names = vec!["(Yeni profil)".to_string()];
                for p in profiles.borrow().iter() {
                    names.push(p.name.clone());
                }
                let str_slices: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                site_combo.set_model(Some(&gtk4::StringList::new(&str_slices)));
            }
        };
        update_combo_model();

        let notebook = gtk4::Notebook::builder().vexpand(true).build();

        // --- Genel ---
        let sec_profile = FormSection::new(Some("Profil"), None);
        let site_name_row = widgets::entry("Yeni Sunucu");
        sec_profile.row("Site adı", &site_name_row);

        let sec_server = FormSection::new(Some("Sunucu"), None);
        let proto_row = widgets::dropdown(PROTOCOLS);
        sec_server.row("Protokol", &proto_row);
        let host_row = widgets::entry("");
        sec_server.row("Adres", &host_row);
        let port_row = widgets::entry("22");
        port_row.set_input_purpose(gtk4::InputPurpose::Digits);
        sec_server.row("Port", &port_row);

        let sec_auth = FormSection::new(Some("Kimlik doğrulama"), None);
        let user_row = widgets::entry("");
        sec_auth.row("Kullanıcı adı", &user_row);
        let password_row = widgets::password_entry();
        sec_auth.row("Parola", &password_row);
        let key_box = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).css_classes(["linked"]).build();
        let key_row = widgets::entry("");
        key_row.set_placeholder_text(Some("~/.ssh/id_rsa"));
        key_box.append(&key_row);
        let btn_browse_key = gtk4::Button::from_icon_name(icons::pick(icons::FOLDER_OPEN));
        btn_browse_key.set_tooltip_text(Some("Anahtar dosyası seç"));
        key_box.append(&btn_browse_key);
        sec_auth.row("SSH anahtarı", &key_box);
        notebook.append_page(&page(&[&sec_profile, &sec_server, &sec_auth]), Some(&gtk4::Label::new(Some("Genel"))));

        // --- Gelişmiş ---
        let sec_paths = FormSection::new(Some("Varsayılan dizinler"), Some("Bağlantı kurulduğunda açılacak klasörler"));
        let def_local = widgets::entry("~/Projeler");
        sec_paths.row("Yerel dizin", &def_local);
        let def_remote = widgets::entry("/var/www/html");
        sec_paths.row("Uzak dizin", &def_remote);
        let (sync_box, _sync_browse) = widgets::check("Eşzamanlı gezinme", Some("Yerel ve uzak tarafta aynı anda dizin değiştir"), false);
        sec_paths.wide(&sync_box);
        let (hidden_box, _hidden_always) = widgets::check("Gizli dosyaları her zaman göster", Some("Nokta (.) ile başlayan dosyaları listele"), false);
        sec_paths.wide(&hidden_box);
        notebook.append_page(&page(&[&sec_paths]), Some(&gtk4::Label::new(Some("Gelişmiş"))));

        // --- Aktarım ---
        let sec_tx = FormSection::new(Some("Aktarım ve hız"), None);
        sec_tx.row("Aktarım modu", &widgets::dropdown(&["Pasif (önerilen)", "Aktif", "Varsayılan"]));
        sec_tx.row("İndirme sınırı", &widgets::dropdown(&["Sınırsız", "500 KB/s", "1 MB/s", "5 MB/s"]));
        sec_tx.row("Yükleme sınırı", &widgets::dropdown(&["Sınırsız", "500 KB/s", "1 MB/s", "5 MB/s"]));
        sec_tx.row("Dosya zaten varsa", &widgets::dropdown(&["Üzerine yaz", "Atla", "Yeniden adlandır", "Sor"]));
        notebook.append_page(&page(&[&sec_tx]), Some(&gtk4::Label::new(Some("Aktarım"))));

        // --- Ağ ---
        let sec_net = FormSection::new(Some("Bağlantı"), None);
        sec_net.row("Karakter kodlaması", &widgets::dropdown(&[
            "Otomatik / UTF-8",
            "ISO-8859-9 (Türkçe Latin-5)",
            "Windows-1254 (Türkçe)",
        ]));
        sec_net.row("Proxy", &widgets::dropdown(&["Yok", "SOCKS5", "HTTP/1.1", "FTP proxy"]));
        notebook.append_page(&page(&[&sec_net]), Some(&gtk4::Label::new(Some("Ağ ve Güvenlik"))));

        let nb_frame = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(16).margin_end(16)
            .vexpand(true)
            .build();
        nb_frame.append(&notebook);
        main_vbox.append(&nb_frame);

        // Dialog buttons
        let action_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .margin_top(12).margin_bottom(12).margin_start(16).margin_end(16)
            .build();
        let btn_close = gtk4::Button::with_mnemonic("_Kapat");
        let w_close = window.clone();
        btn_close.connect_clicked(move |_| w_close.close());
        action_bar.append(&btn_close);
        let btn_connect = gtk4::Button::builder()
            .label("Bağlan")
            .css_classes(["suggested-action"])
            .build();
        action_bar.append(&btn_connect);
        main_vbox.append(&action_bar);
        window.set_child(Some(&main_vbox));
        window.set_default_widget(Some(&btn_connect));

        // SSH key file picker
        let key_pick = key_row.clone();
        let w_key = window.clone();
        btn_browse_key.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::builder().title("SSH özel anahtarını seç").modal(true).build();
            let key_pick = key_pick.clone();
            dialog.open(Some(&w_key), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        key_pick.set_text(&p.display().to_string());
                    }
                }
            });
        });

        // Port follows protocol
        let port_auto = port_row.clone();
        proto_row.connect_selected_notify(move |dd| {
            port_auto.set_text(if dd.selected() == 0 { "22" } else { "21" });
        });

        // Combo select logic
        let site_name_c = site_name_row.clone();
        let host_c = host_row.clone();
        let port_c = port_row.clone();
        let user_c = user_row.clone();
        let pass_c = password_row.clone();
        let proto_c = proto_row.clone();
        let prof_c = Rc::clone(&profiles);

        site_combo.connect_selected_notify(move |combo| {
            let idx = combo.selected() as usize;
            if idx == 0 || idx > prof_c.borrow().len() {
                site_name_c.set_text("Yeni Sunucu");
                host_c.set_text("");
                proto_c.set_selected(0);
                port_c.set_text("22");
                user_c.set_text("");
                pass_c.set_text("");
            } else {
                let p = prof_c.borrow()[idx - 1].clone();
                site_name_c.set_text(&p.name);
                host_c.set_text(&p.host);
                proto_c.set_selected(match p.proto {
                    Protocol::Sftp => 0,
                    Protocol::Ftp => 1,
                    Protocol::Ftps => 2,
                });
                // set after protocol: the protocol handler resets the port
                port_c.set_text(&p.port.to_string());
                user_c.set_text(&p.user);
                pass_c.set_text(p.password.as_deref().unwrap_or(""));
            }
        });

        // Save logic
        let site_name_s = site_name_row.clone();
        let host_s = host_row.clone();
        let port_s = port_row.clone();
        let user_s = user_row.clone();
        let pass_s = password_row.clone();
        let proto_s = proto_row.clone();
        let def_rem_s = def_remote.clone();
        let def_loc_s = def_local.clone();
        let prof_s = Rc::clone(&profiles);
        let store_s = Rc::clone(&store);
        let up_combo_s = update_combo_model.clone();

        btn_save.connect_clicked(move |_| {
            let name = site_name_s.text().trim().to_string();
            let host = host_s.text().trim().to_string();
            if host.is_empty() {
                host_s.add_css_class("error");
                return;
            }
            host_s.remove_css_class("error");
            let user = user_s.text().trim().to_string();

            let id = if let Some(p) = prof_s.borrow().iter().find(|p| p.name == name) {
                p.id.clone()
            } else {
                format!("{}_{}", host, user)
            };

            let profile = ServerProfile {
                id,
                name: if name.is_empty() { host.clone() } else { name },
                host,
                port: port_s.text().parse().unwrap_or(22),
                user,
                password: Some(pass_s.text().to_string()),
                proto: proto_from_index(proto_s.selected()),
                default_remote_dir: Some(def_rem_s.text().trim().to_string()),
                default_local_dir: Some(def_loc_s.text().trim().to_string()),
            };

            store_s.borrow().add_server(profile);
            *prof_s.borrow_mut() = store_s.borrow().get_all();
            up_combo_s();
        });

        // Delete logic
        let prof_d = Rc::clone(&profiles);
        let store_d = Rc::clone(&store);
        let site_combo_d = site_combo.clone();
        let up_combo_d = update_combo_model.clone();

        btn_del.connect_clicked(move |_| {
            let idx = site_combo_d.selected() as usize;
            if idx > 0 && idx <= prof_d.borrow().len() {
                let id = prof_d.borrow()[idx - 1].id.clone();
                store_d.borrow().delete_server(&id);
                *prof_d.borrow_mut() = store_d.borrow().get_all();
                up_combo_d();
                site_combo_d.set_selected(0);
            }
        });

        // Connect button logic
        let w_conn = window.clone();
        btn_connect.connect_clicked(move |_| {
            let host = host_row.text().trim().to_string();
            if host.is_empty() {
                host_row.add_css_class("error");
                return;
            }
            let user = user_row.text().trim().to_string();
            let name = site_name_row.text().trim().to_string();
            let profile = ServerProfile {
                id: format!("{}_{}", host, user),
                name: if name.is_empty() { host.clone() } else { name },
                host,
                port: port_row.text().parse().unwrap_or(22),
                user,
                password: Some(password_row.text().to_string()),
                proto: proto_from_index(proto_row.selected()),
                default_remote_dir: Some(def_remote.text().trim().to_string()),
                default_local_dir: Some(def_local.text().trim().to_string()),
            };

            on_connect(profile);
            w_conn.close();
        });

        window.present();
    }
}
