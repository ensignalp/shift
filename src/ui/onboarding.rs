use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use crate::models::{Protocol, ServerProfile};
use crate::server_store::ServerStore;

pub struct OnboardingView {
    pub container: gtk4::Box,
    pub reload_saved: Rc<dyn Fn()>,
}

impl OnboardingView {
    pub fn new<F1, F2>(on_connect: F1, on_open_site_manager: F2) -> Self
    where
        F1: Fn(ServerProfile) + 'static + Clone,
        F2: Fn() + 'static + Clone,
    {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();

        let viewport = gtk4::Viewport::builder()
            .scroll_to_focus(false)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Flawlessly centered vertically and horizontally
        let center_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(20)
            .margin_end(20)
            .vexpand(true)
            .hexpand(true)
            .build();

        let clamp = libadwaita::Clamp::builder()
            .maximum_size(580)
            .tightening_threshold(480)
            .build();

        let content_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();

        // 1. Branding: 95px Crisp Logo on left, Title & Subtitle on right
        let hero_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(16)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .margin_top(4)
            .margin_bottom(6)
            .build();

        let logo_bytes = glib::Bytes::from_static(include_bytes!("../../data/shift_logo_95.png"));
        let hero_logo = match gtk4::gdk::Texture::from_bytes(&logo_bytes) {
            Ok(texture) => gtk4::Image::from_paintable(Some(&texture)),
            Err(_) => gtk4::Image::from_icon_name("com.ensignsoft.Shift"),
        };
        hero_logo.set_pixel_size(95);
        hero_logo.set_size_request(95, 95);
        hero_logo.add_css_class("hero-logo-img");
        hero_box.append(&hero_logo);

        let title_vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .valign(gtk4::Align::Center)
            .build();

        let title_label = gtk4::Label::builder()
            .label("Shift")
            .css_classes(["title-1", "bold"])
            .xalign(0.0)
            .build();
        title_vbox.append(&title_label);

        let desc_label = gtk4::Label::builder()
            .label("Ensignsoft Güvencesiyle Bulut Dosya Aktarım İstemcisi")
            .css_classes(["caption", "dim-label"])
            .wrap(true)
            .xalign(0.0)
            .build();
        title_vbox.append(&desc_label);
        hero_box.append(&title_vbox);
        content_box.append(&hero_box);

        // 2. Mode Switcher
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let switcher = gtk4::StackSwitcher::builder()
            .stack(&stack)
            .css_classes(["linked", "pill"])
            .halign(gtk4::Align::Center)
            .margin_top(2)
            .margin_bottom(4)
            .build();
        content_box.append(&switcher);

        // TAB 1: Quick Connect
        let tab_quick = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .build();

        let card_group = libadwaita::PreferencesGroup::new();

        let proto_row = libadwaita::ComboRow::builder()
            .title("Protokol")
            .model(&gtk4::StringList::new(&[
                "SFTP - SSH Dosya Aktarım Protokolü (Port 22)",
                "FTP - Standart Dosya Aktarım Protokolü (Port 21)",
                "FTPS - Güvenli TLS Şifrelemeli FTP (Port 21)"
            ]))
            .build();
        proto_row.add_prefix(&gtk4::Image::from_icon_name("network-workgroup-symbolic"));
        card_group.add(&proto_row);

        let host_row = libadwaita::EntryRow::builder()
            .title("Sunucu Adresi (Host / IP)")
            .build();
        host_row.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER)));
        card_group.add(&host_row);

        let port_row = libadwaita::EntryRow::builder()
            .title("Bağlantı Portu")
            .text("22")
            .build();
        port_row.add_prefix(&gtk4::Image::from_icon_name("network-workgroup-symbolic"));
        card_group.add(&port_row);

        let user_row = libadwaita::EntryRow::builder()
            .title("Kullanıcı Adı")
            .build();
        user_row.add_prefix(&gtk4::Image::from_icon_name("avatar-default-symbolic"));
        card_group.add(&user_row);

        let password_row = libadwaita::PasswordEntryRow::builder()
            .title("Parola")
            .build();
        password_row.add_prefix(&gtk4::Image::from_icon_name("dialog-password-symbolic"));
        card_group.add(&password_row);

        let remember_row = libadwaita::SwitchRow::builder()
            .title("Kayıtlı Sunuculara Ekle")
            .subtitle("Beni hatırla ve sunucu profilini sakla")
            .active(true)
            .build();
        remember_row.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SAVE)));
        card_group.add(&remember_row);

        tab_quick.append(&card_group);

        let btn_primary = gtk4::Button::builder()
            .css_classes(["suggested-action", "pill"])
            .height_request(46)
            .margin_top(4)
            .build();

        let btn_content = libadwaita::ButtonContent::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::CONNECT))
            .label("Sunucuya Bağlan")
            .build();
        btn_primary.set_child(Some(&btn_content));
        tab_quick.append(&btn_primary);

        stack.add_titled(&tab_quick, Some("quick"), "Hızlı Bağlantı");

        // TAB 2: Saved Servers Container
        let tab_saved = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .build();

        let saved_container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .build();
        tab_saved.append(&saved_container);
        stack.add_titled(&tab_saved, Some("saved"), "Kayıtlı Sunucular");

        content_box.append(&stack);
        clamp.set_child(Some(&content_box));
        center_box.append(&clamp);
        viewport.set_child(Some(&center_box));
        scrolled.set_child(Some(&viewport));
        container.append(&scrolled);

        // Port auto-update on protocol change
        let port_clone = port_row.clone();
        proto_row.connect_selected_notify(move |combo| {
            if combo.selected() == 0 {
                port_clone.set_text("22");
            } else {
                port_clone.set_text("21");
            }
        });

        // Dynamic Saved Servers Reload Closure
        let saved_box = saved_container.clone();
        let on_conn_for_reload = on_connect.clone();
        let on_sm_for_reload = on_open_site_manager.clone();
        let content_box_for_modal = content_box.clone();

        let reload_saved_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let reload_saved_fn_weak = Rc::clone(&reload_saved_fn);

        let reload_logic = {
            let saved_box = saved_box.clone();
            let on_conn = on_conn_for_reload.clone();
            let on_sm = on_sm_for_reload.clone();
            let content_box = content_box_for_modal.clone();
            let reload_saved_fn_weak = reload_saved_fn_weak.clone();

            move || {
                // Clear existing widgets
                while let Some(child) = saved_box.first_child() {
                    saved_box.remove(&child);
                }

                let saved_group = libadwaita::PreferencesGroup::new();
                let store = ServerStore::new();
                let all_servers = store.get_all();

                for srv in all_servers {
                    let row = libadwaita::ActionRow::builder()
                        .title(&srv.name)
                        .subtitle(&format!("{} • {}@{}:{}", srv.proto.as_str().to_uppercase(), srv.user, srv.host, srv.port))
                        .activatable(true)
                        .build();
                    row.add_prefix(&gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER)));

                    // Connect button
                    let btn_conn = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FORWARD));
                    btn_conn.add_css_class("flat");
                    btn_conn.add_css_class("circular");
                    btn_conn.set_tooltip_text(Some("Hemen Bağlan"));
                    btn_conn.set_valign(gtk4::Align::Center);
                    let srv_c1 = srv.clone();
                    let on_c1 = on_conn.clone();
                    btn_conn.connect_clicked(move |_| {
                        on_c1(srv_c1.clone());
                    });
                    row.add_suffix(&btn_conn);

                    // Edit button
                    let btn_edit = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::RENAME));
                    btn_edit.add_css_class("flat");
                    btn_edit.add_css_class("circular");
                    btn_edit.set_tooltip_text(Some("Profili Düzenle"));
                    btn_edit.set_valign(gtk4::Align::Center);
                    let srv_c2 = srv.clone();
                    let root_widget = content_box.clone();
                    let reload_ref = reload_saved_fn_weak.clone();
                    btn_edit.connect_clicked(move |_| {
                        let parent_win = root_widget.root().and_downcast::<gtk4::Window>();
                        let dialog = libadwaita::MessageDialog::builder()
                            .heading("Sunucu Profilini Düzenle")
                            .body("Sunucu profil bilgilerini güncelleyin.")
                            .build();
                        if let Some(ref p) = parent_win {
                            dialog.set_transient_for(Some(p));
                        }

                        let box_extra = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
                        box_extra.set_margin_top(12);
                        box_extra.set_margin_bottom(12);

                        let name_entry = libadwaita::EntryRow::builder().title("Profil Adı").text(&srv_c2.name).build();
                        box_extra.append(&name_entry);

                        let host_entry = libadwaita::EntryRow::builder().title("Sunucu (Host)").text(&srv_c2.host).build();
                        box_extra.append(&host_entry);

                        let port_entry = libadwaita::EntryRow::builder().title("Port").text(&srv_c2.port.to_string()).build();
                        box_extra.append(&port_entry);

                        let user_entry = libadwaita::EntryRow::builder().title("Kullanıcı Adı").text(&srv_c2.user).build();
                        box_extra.append(&user_entry);

                        let pwd_entry = libadwaita::PasswordEntryRow::builder().title("Parola").build();
                        if let Some(ref p) = srv_c2.password {
                            pwd_entry.set_text(p);
                        }
                        box_extra.append(&pwd_entry);

                        dialog.set_extra_child(Some(&box_extra));
                        dialog.add_response("cancel", "İptal");
                        dialog.add_response("save", "Kaydet");
                        dialog.set_response_appearance("save", libadwaita::ResponseAppearance::Suggested);

                        let srv_for_save = srv_c2.clone();
                        let reload_ref2 = reload_ref.clone();
                        dialog.connect_response(None, move |_, resp| {
                            if resp == "save" {
                                let p_val = port_entry.text().parse::<u16>().unwrap_or(21);
                                let updated = ServerProfile {
                                    id: srv_for_save.id.clone(),
                                    name: name_entry.text().trim().to_string(),
                                    host: host_entry.text().trim().to_string(),
                                    user: user_entry.text().trim().to_string(),
                                    password: Some(pwd_entry.text().to_string()),
                                    port: p_val,
                                    proto: srv_for_save.proto,
                                    default_remote_dir: srv_for_save.default_remote_dir.clone(),
                                    default_local_dir: srv_for_save.default_local_dir.clone(),
                                };
                                let store = ServerStore::new();
                                store.update_server(updated);
                                if let Some(ref r) = *reload_ref2.borrow() {
                                    r();
                                }
                            }
                        });
                        dialog.present();
                    });
                    row.add_suffix(&btn_edit);

                    // Delete button
                    let btn_del = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::DELETE));
                    btn_del.add_css_class("flat");
                    btn_del.add_css_class("circular");
                    btn_del.set_tooltip_text(Some("Profili Sil"));
                    btn_del.set_valign(gtk4::Align::Center);
                    let srv_c3 = srv.clone();
                    let root_widget_del = content_box.clone();
                    let reload_ref_del = reload_saved_fn_weak.clone();
                    btn_del.connect_clicked(move |_| {
                        let parent_win = root_widget_del.root().and_downcast::<gtk4::Window>();
                        let dialog = libadwaita::MessageDialog::builder()
                            .heading("Sunucu Profilini Sil?")
                            .body(&format!("'{}' profilini silmek istediğinize emin misiniz?", srv_c3.name))
                            .build();
                        if let Some(ref p) = parent_win {
                            dialog.set_transient_for(Some(p));
                        }
                        dialog.add_response("cancel", "Vazgeç");
                        dialog.add_response("delete", "Sil");
                        dialog.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);

                        let srv_id = srv_c3.id.clone();
                        let reload_ref_del2 = reload_ref_del.clone();
                        dialog.connect_response(None, move |_, resp| {
                            if resp == "delete" {
                                let store = ServerStore::new();
                                store.delete_server(&srv_id);
                                if let Some(ref r) = *reload_ref_del2.borrow() {
                                    r();
                                }
                            }
                        });
                        dialog.present();
                    });
                    row.add_suffix(&btn_del);

                    // Row activated connects
                    let srv_c4 = srv.clone();
                    let on_c4 = on_conn.clone();
                    row.connect_activated(move |_| {
                        on_c4(srv_c4.clone());
                    });

                    saved_group.add(&row);
                }

                // Site Manager button row
                let sm_row = libadwaita::ActionRow::builder()
                    .title("Site Yöneticisini Aç...")
                    .subtitle("Tüm kayıtlı sunucuları detaylı yönet")
                    .activatable(true)
                    .build();
                sm_row.add_prefix(&gtk4::Image::from_icon_name("preferences-system-symbolic"));
                let btn_sm = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FORWARD));
                btn_sm.add_css_class("flat");
                btn_sm.add_css_class("circular");
                btn_sm.set_valign(gtk4::Align::Center);
                sm_row.add_suffix(&btn_sm);

                let on_sm_clone = on_sm.clone();
                sm_row.connect_activated(move |_| {
                    on_sm_clone();
                });
                saved_group.add(&sm_row);

                saved_box.append(&saved_group);
            }
        };

        let rc_reload: Rc<dyn Fn()> = Rc::new(reload_logic);
        *reload_saved_fn.borrow_mut() = Some(rc_reload.clone());
        rc_reload();

        // Connect button logic
        let on_conn = on_connect.clone();
        let proto_clone = proto_row.clone();
        let port_for_conn = port_row.clone();
        let rc_reload_for_btn = rc_reload.clone();
        btn_primary.connect_clicked(move |_| {
            let host = host_row.text().trim().to_string();
            if host.is_empty() {
                host_row.add_css_class("error");
                return;
            }
            host_row.remove_css_class("error");

            let user = user_row.text().trim().to_string();
            let user = if user.is_empty() { "anonymous".to_string() } else { user };
            let pass = password_row.text().to_string();
            let port: u16 = port_for_conn.text().parse().unwrap_or(22);

            let proto = match proto_clone.selected() {
                1 => Protocol::Ftp,
                2 => Protocol::Ftps,
                _ => Protocol::Sftp,
            };

            let profile = ServerProfile {
                id: format!("{}_{}", host, user),
                name: host.clone(),
                host,
                port,
                user,
                password: Some(pass),
                proto,
                default_remote_dir: Some("/".to_string()),
                default_local_dir: None,
            };

            if remember_row.is_active() {
                let store = ServerStore::new();
                store.add_server(profile.clone());
                rc_reload_for_btn();
            }

            on_conn(profile);
        });

        Self {
            container,
            reload_saved: rc_reload,
        }
    }
}
