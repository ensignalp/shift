use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use crate::models::{Protocol, ServerProfile};
use crate::server_store::ServerStore;

pub struct SiteManagerWindow;

impl SiteManagerWindow {
    pub fn show<F>(parent: &libadwaita::ApplicationWindow, on_connect: F)
    where
        F: Fn(ServerProfile) + 'static + Clone,
    {
        let window = libadwaita::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Shift - Site Yöneticisi")
            .default_width(563)
            .default_height(860)
            .width_request(563)
            .height_request(860)
            .build();

        let store = Rc::new(RefCell::new(ServerStore::new()));
        let profiles = Rc::new(RefCell::new(store.borrow().get_all()));

        let main_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Header with Logo beside Name
        let header = libadwaita::HeaderBar::new();
        header.add_css_class("flat");

        let title_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        title_box.set_halign(gtk4::Align::Center);
        title_box.set_valign(gtk4::Align::Center);

        let logo_bytes = glib::Bytes::from_static(include_bytes!("../../data/shift_logo_95.png"));
        let logo_img = match gtk4::gdk::Texture::from_bytes(&logo_bytes) {
            Ok(texture) => gtk4::Image::from_paintable(Some(&texture)),
            Err(_) => gtk4::Image::from_icon_name("com.ensignsoft.Shift"),
        };
        logo_img.set_pixel_size(24);
        logo_img.set_size_request(24, 24);
        title_box.append(&logo_img);

        let title_lbl = gtk4::Label::builder()
            .label("Site Yöneticisi")
            .css_classes(["title-4", "bold"])
            .build();
        title_box.append(&title_lbl);

        header.set_title_widget(Some(&title_box));
        main_vbox.append(&header);

        // ViewStack for 4 tabs
        let view_stack = libadwaita::ViewStack::new();
        view_stack.set_vexpand(true);
        view_stack.set_hexpand(true);

        // Top Switcher
        let switcher = libadwaita::ViewSwitcher::builder()
            .stack(&view_stack)
            .policy(libadwaita::ViewSwitcherPolicy::Wide)
            .halign(gtk4::Align::Center)
            .margin_top(4)
            .margin_bottom(6)
            .css_classes(["top-switcher"])
            .build();
        main_vbox.append(&switcher);

        // --- PAGE 1: GENEL ---
        let page_general = libadwaita::PreferencesPage::new();
        let group_profile = libadwaita::PreferencesGroup::builder()
            .title("Site Profili")
            .description("Kayıtlı sunucuyu seçin, düzenleyin veya yeni profil oluşturun")
            .build();

        let site_combo = libadwaita::ComboRow::builder()
            .title("Kayıtlı Siteler")
            .build();
        site_combo.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER)));

        let update_combo_model = {
            let site_combo = site_combo.clone();
            let profiles = Rc::clone(&profiles);
            move || {
                let mut names = vec!["(Yeni Profil Oluştur)".to_string()];
                for p in profiles.borrow().iter() {
                    names.push(p.name.clone());
                }
                let str_slices: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                site_combo.set_model(Some(&gtk4::StringList::new(&str_slices)));
            }
        };
        update_combo_model();

        let site_name_row = libadwaita::EntryRow::builder()
            .title("Profil / Site Adı")
            .text("Yeni Sunucu")
            .build();
        site_name_row.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::RENAME)));

        let proto_row = libadwaita::ComboRow::builder()
            .title("Protokol")
            .model(&gtk4::StringList::new(&[
                "SFTP - SSH Dosya Aktarım Protokolü (Port 22)",
                "FTP - Standart Dosya Aktarım Protokolü (Port 21)",
                "FTPS - Güvenli TLS Şifrelemeli FTP (Port 21)"
            ]))
            .build();
        proto_row.add_prefix(&gtk4::Image::from_icon_name("network-workgroup-symbolic"));

        let host_row = libadwaita::EntryRow::builder()
            .title("Sunucu Adresi")
            .build();
        host_row.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER)));

        let port_row = libadwaita::EntryRow::builder()
            .title("Port")
            .text("22")
            .build();
        port_row.add_prefix(&gtk4::Image::from_icon_name("network-workgroup-symbolic"));

        let user_row = libadwaita::EntryRow::builder()
            .title("Kullanıcı Adı")
            .build();
        user_row.add_prefix(&gtk4::Image::from_icon_name("avatar-default-symbolic"));

        let password_row = libadwaita::PasswordEntryRow::builder()
            .title("Parola")
            .build();
        password_row.add_prefix(&gtk4::Image::from_icon_name("dialog-password-symbolic"));

        let key_row = libadwaita::EntryRow::builder()
            .title("SSH Özel Anahtarı (.pem / .id_rsa)")
            .build();
        key_row.add_prefix(&gtk4::Image::from_icon_name("dialog-password-symbolic"));
        let btn_browse_key = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FOLDER_OPEN));
        btn_browse_key.add_css_class("flat");
        btn_browse_key.add_css_class("circular");
        key_row.add_suffix(&btn_browse_key);

        // Save & Delete buttons
        let btn_save = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SAVE));
        btn_save.set_tooltip_text(Some("Bu Profili Kaydet / Güncelle"));
        btn_save.add_css_class("flat");
        btn_save.add_css_class("circular");
        site_combo.add_suffix(&btn_save);

        let btn_del = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::DELETE));
        btn_del.set_tooltip_text(Some("Bu Profili Sil"));
        btn_del.add_css_class("flat");
        btn_del.add_css_class("circular");
        site_combo.add_suffix(&btn_del);

        group_profile.add(&site_combo);
        group_profile.add(&site_name_row);
        page_general.add(&group_profile);

        let group_server = libadwaita::PreferencesGroup::builder()
            .title("Sunucu Bilgileri")
            .build();
        group_server.add(&proto_row);
        group_server.add(&host_row);
        group_server.add(&port_row);
        page_general.add(&group_server);

        let group_auth = libadwaita::PreferencesGroup::builder()
            .title("Kimlik Doğrulama")
            .build();
        group_auth.add(&user_row);
        group_auth.add(&password_row);
        group_auth.add(&key_row);
        page_general.add(&group_auth);

        let scroll_gen = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&page_general)
            .build();
        view_stack.add_titled_with_icon(&scroll_gen, Some("general"), "Genel", crate::ui::icons::pick(crate::ui::icons::SERVER));

        // --- PAGE 2: GELİŞMİŞ ---
        let page_adv = libadwaita::PreferencesPage::new();
        let group_paths = libadwaita::PreferencesGroup::builder()
            .title("Varsayılan Dizinler")
            .description("Bağlantı kurulduğunda otomatik açılacak klasörler")
            .build();

        let def_local = libadwaita::EntryRow::builder()
            .title("Varsayılan Yerel Dizin")
            .text("~/Projeler")
            .build();
        def_local.add_prefix(&gtk4::Image::from_icon_name("drive-harddisk-symbolic"));
        group_paths.add(&def_local);

        let def_remote = libadwaita::EntryRow::builder()
            .title("Varsayılan Uzak Dizin")
            .text("/var/www/html")
            .build();
        def_remote.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FOLDER)));
        group_paths.add(&def_remote);

        let sync_browse = libadwaita::SwitchRow::builder()
            .title("Eşzamanlı Gezinme (Synchronized Browsing)")
            .subtitle("Yerel ve uzak tarafta aynı anda dizin değiştir")
            .build();
        sync_browse.add_prefix(&gtk4::Image::from_icon_name("insert-link-symbolic"));
        group_paths.add(&sync_browse);

        let hidden_always = libadwaita::SwitchRow::builder()
            .title("Gizli Dosyaları Her Zaman Göster")
            .subtitle("Nokta (.) ile başlayan sistem dosyalarını listele")
            .build();
        hidden_always.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SHOW_HIDDEN)));
        group_paths.add(&hidden_always);

        page_adv.add(&group_paths);

        let scroll_adv = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&page_adv)
            .build();
        view_stack.add_titled_with_icon(&scroll_adv, Some("advanced"), "Gelişmiş", "preferences-system-symbolic");

        // --- PAGE 3: AKTARIM AYARLARI ---
        let page_tx = libadwaita::PreferencesPage::new();
        let group_tx = libadwaita::PreferencesGroup::builder()
            .title("Aktarım ve Hız Yapılandırması")
            .build();

        let tx_mode = libadwaita::ComboRow::builder()
            .title("Aktarım Modu")
            .model(&gtk4::StringList::new(&["Pasif Mod (Önerilen)", "Aktif Mod", "Varsayılan"]))
            .build();
        tx_mode.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::REFRESH)));
        group_tx.add(&tx_mode);

        let dl_limit = libadwaita::ComboRow::builder()
            .title("İndirme Hız Sınırı")
            .model(&gtk4::StringList::new(&["Sınırsız (Maksimum)", "500 KB/s", "1 MB/s", "5 MB/s"]))
            .build();
        dl_limit.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::DOWNLOAD)));
        group_tx.add(&dl_limit);

        let ul_limit = libadwaita::ComboRow::builder()
            .title("Yükleme Hız Sınırı")
            .model(&gtk4::StringList::new(&["Sınırsız (Maksimum)", "500 KB/s", "1 MB/s", "5 MB/s"]))
            .build();
        ul_limit.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::UPLOAD)));
        group_tx.add(&ul_limit);

        let exist_action = libadwaita::ComboRow::builder()
            .title("Hedefte Aynı Dosya Varsa")
            .model(&gtk4::StringList::new(&["Üzerine Yaz", "Atla", "Yeniden Adlandır", "Sor"]))
            .build();
        exist_action.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FILE)));
        group_tx.add(&exist_action);

        page_tx.add(&group_tx);
        let scroll_tx = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&page_tx)
            .build();
        view_stack.add_titled_with_icon(&scroll_tx, Some("transfer"), "Aktarım Ayarları", crate::ui::icons::pick(crate::ui::icons::DOWNLOAD));

        // --- PAGE 4: AĞ VE GÜVENLİK ---
        let page_net = libadwaita::PreferencesPage::new();
        let group_net = libadwaita::PreferencesGroup::builder()
            .title("Zaman Aşımı ve Canlı Tutma")
            .build();

        let charset_row = libadwaita::ComboRow::builder()
            .title("Karakter Kodlaması")
            .model(&gtk4::StringList::new(&[
                "Otomatik Algıla / UTF-8 Zorla",
                "ISO-8859-9 (Türkçe Latin-5)",
                "Windows-1254 (Türkçe)"
            ]))
            .build();
        charset_row.add_prefix(&gtk4::Image::from_icon_name("network-workgroup-symbolic"));
        group_net.add(&charset_row);

        let proxy_type_row = libadwaita::ComboRow::builder()
            .title("Proxy Türü")
            .model(&gtk4::StringList::new(&["Proxy Yok", "SOCKS5", "HTTP / 1.1 Proxy", "FTP Proxy"]))
            .build();
        proxy_type_row.add_prefix(&gtk4::Image::from_icon_name("security-high-symbolic"));
        group_net.add(&proxy_type_row);

        page_net.add(&group_net);
        let scroll_net = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&page_net)
            .build();
        view_stack.add_titled_with_icon(&scroll_net, Some("network"), "Ağ ve Güvenlik", "security-high-symbolic");

        main_vbox.append(&view_stack);

        // Bottom Action Bar: Connect & Close
        let action_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        action_bar.set_halign(gtk4::Align::Center);
        action_bar.set_margin_top(8);
        action_bar.set_margin_bottom(12);

        let btn_connect = gtk4::Button::builder()
            .css_classes(["suggested-action", "pill"])
            .width_request(220)
            .height_request(44)
            .build();
        let conn_content = libadwaita::ButtonContent::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::CONNECT))
            .label("Seçili Sunucuya Bağlan")
            .build();
        btn_connect.set_child(Some(&conn_content));
        action_bar.append(&btn_connect);

        let btn_close = gtk4::Button::builder()
            .label("Kapat")
            .css_classes(["flat", "pill"])
            .width_request(90)
            .height_request(44)
            .build();
        let w_close = window.clone();
        btn_close.connect_clicked(move |_| w_close.close());
        action_bar.append(&btn_close);

        main_vbox.append(&action_bar);
        window.set_content(Some(&main_vbox));

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
                port_c.set_text("22");
                user_c.set_text("");
                pass_c.set_text("");
                proto_c.set_selected(0);
            } else {
                let p = &prof_c.borrow()[idx - 1];
                site_name_c.set_text(&p.name);
                host_c.set_text(&p.host);
                port_c.set_text(&p.port.to_string());
                user_c.set_text(&p.user);
                pass_c.set_text(p.password.as_deref().unwrap_or(""));
                let p_idx = match p.proto {
                    Protocol::Sftp => 0,
                    Protocol::Ftp => 1,
                    Protocol::Ftps => 2,
                };
                proto_c.set_selected(p_idx);
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
            if host.is_empty() { return; }
            let port: u16 = port_s.text().parse().unwrap_or(22);
            let user = user_s.text().trim().to_string();
            let pass = pass_s.text().to_string();
            let proto = match proto_s.selected() {
                1 => Protocol::Ftp,
                2 => Protocol::Ftps,
                _ => Protocol::Sftp,
            };

            let id = if let Some(p) = prof_s.borrow().iter().find(|p| p.name == name) {
                p.id.clone()
            } else {
                format!("{}_{}", host, user)
            };

            let profile = ServerProfile {
                id,
                name: if name.is_empty() { host.clone() } else { name },
                host,
                port,
                user,
                password: Some(pass),
                proto,
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
        let host_conn = host_row.clone();
        let port_conn = port_row.clone();
        let user_conn = user_row.clone();
        let pass_conn = password_row.clone();
        let proto_conn = proto_row.clone();
        let site_name_conn = site_name_row.clone();
        let def_rem_conn = def_remote.clone();
        let def_loc_conn = def_local.clone();
        let w_conn = window.clone();

        btn_connect.connect_clicked(move |_| {
            let host = host_conn.text().trim().to_string();
            if host.is_empty() { return; }
            let port: u16 = port_conn.text().parse().unwrap_or(22);
            let user = user_conn.text().trim().to_string();
            let pass = pass_conn.text().to_string();
            let proto = match proto_conn.selected() {
                1 => Protocol::Ftp,
                2 => Protocol::Ftps,
                _ => Protocol::Sftp,
            };

            let name = site_name_conn.text().trim().to_string();
            let profile = ServerProfile {
                id: format!("{}_{}", host, user),
                name: if name.is_empty() { host.clone() } else { name },
                host,
                port,
                user,
                password: Some(pass),
                proto,
                default_remote_dir: Some(def_rem_conn.text().trim().to_string()),
                default_local_dir: Some(def_loc_conn.text().trim().to_string()),
            };

            on_connect(profile);
            w_conn.close();
        });

        window.present();
    }
}
