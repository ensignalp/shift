use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use crate::models::{Protocol, ServerProfile};
use crate::server_store::ServerStore;
use crate::ui::icons;
use crate::ui::widgets::{self, Field, FormSection};

pub struct OnboardingView {
    pub container: gtk4::Box,
    pub reload_saved: Rc<dyn Fn()>,
}

pub const PROTOCOLS: &[&str] = &["SFTP (SSH, port 22)", "FTP (port 21)", "FTPS (TLS, port 21)"];

pub fn proto_from_index(i: u32) -> Protocol {
    match i {
        1 => Protocol::Ftp,
        2 => Protocol::Ftps,
        _ => Protocol::Sftp,
    }
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
            .build();

        let content_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .width_request(420)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // Branding
        let hero_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk4::Align::Center)
            .build();
        let logo_bytes = glib::Bytes::from_static(include_bytes!("../../data/shift_logo_95.png"));
        let hero_logo = match gtk4::gdk::Texture::from_bytes(&logo_bytes) {
            Ok(texture) => gtk4::Image::from_paintable(Some(&texture)),
            Err(_) => gtk4::Image::from_icon_name("com.ensignsoft.Shift"),
        };
        hero_logo.set_pixel_size(64);
        hero_box.append(&hero_logo);
        let title_vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .valign(gtk4::Align::Center)
            .build();
        title_vbox.append(&gtk4::Label::builder().label("Shift").css_classes(["title-1"]).xalign(0.0).build());
        title_vbox.append(&gtk4::Label::builder()
            .label("FTP, FTPS ve SFTP dosya aktarım istemcisi")
            .css_classes(["dim-label"])
            .xalign(0.0)
            .build());
        hero_box.append(&title_vbox);
        content_box.append(&hero_box);

        // Mode switcher
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .vhomogeneous(false)
            .build();
        let switcher = gtk4::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk4::Align::Center)
            .build();
        content_box.append(&switcher);

        // TAB 1: Quick connect
        let tab_quick = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        let form = FormSection::new(None, None);
        let proto_dd = widgets::dropdown(PROTOCOLS);
        form.row("Protokol", &proto_dd);
        let host_entry = widgets::entry("");
        host_entry.set_placeholder_text(Some("ornek.com veya 192.168.1.10"));
        host_entry.set_activates_default(true);
        form.row("Sunucu", &host_entry);
        let port_entry = widgets::entry("22");
        port_entry.set_input_purpose(gtk4::InputPurpose::Digits);
        port_entry.set_activates_default(true);
        form.row("Port", &port_entry);
        let user_entry = widgets::entry("");
        user_entry.set_placeholder_text(Some("anonymous"));
        user_entry.set_activates_default(true);
        form.row("Kullanıcı adı", &user_entry);
        let password_entry = widgets::password_entry();
        password_entry.set_activates_default(true);
        form.row("Parola", &password_entry);
        let (remember_box, remember_cb) = widgets::check("Kayıtlı sunuculara ekle", None, true);
        form.wide(&remember_box);
        tab_quick.append(&form.root);

        let btn_primary = gtk4::Button::builder()
            .label("Bağlan")
            .css_classes(["suggested-action"])
            .halign(gtk4::Align::End)
            .build();
        tab_quick.append(&btn_primary);
        stack.add_titled(&tab_quick, Some("quick"), "Hızlı Bağlantı");

        // TAB 2: Saved servers
        let tab_saved = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        let saved_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        let saved_frame = gtk4::Frame::builder().child(&saved_list).build();
        tab_saved.append(&saved_frame);
        let empty_label = gtk4::Label::builder()
            .label("Henüz kayıtlı sunucu yok.")
            .css_classes(["dim-label"])
            .build();
        tab_saved.append(&empty_label);
        let btn_open_sm = gtk4::Button::builder()
            .label("Site Yöneticisini Aç…")
            .halign(gtk4::Align::End)
            .build();
        let on_sm = on_open_site_manager.clone();
        btn_open_sm.connect_clicked(move |_| on_sm());
        tab_saved.append(&btn_open_sm);
        stack.add_titled(&tab_saved, Some("saved"), "Kayıtlı Sunucular");

        content_box.append(&stack);
        scrolled.set_child(Some(&content_box));
        container.append(&scrolled);

        // Enter in the quick-connect form triggers "Bağlan"
        let btn_default = btn_primary.clone();
        content_box.connect_realize(move |w| {
            if let Some(win) = w.root().and_downcast::<gtk4::Window>() {
                win.set_default_widget(Some(&btn_default));
            }
        });

        // Port auto-update on protocol change
        let port_clone = port_entry.clone();
        proto_dd.connect_selected_notify(move |dd| {
            port_clone.set_text(if dd.selected() == 0 { "22" } else { "21" });
        });

        // Saved servers list (rebuilt on demand)
        let reload_saved_fn: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let row_targets: Rc<RefCell<Vec<ServerProfile>>> = Rc::new(RefCell::new(Vec::new()));
        let reload_logic = {
            let saved_list = saved_list.clone();
            let saved_frame = saved_frame.clone();
            let empty_label = empty_label.clone();
            let on_conn = on_connect.clone();
            let root_ref = content_box.clone();
            let reload_ref = Rc::clone(&reload_saved_fn);
            let row_targets = Rc::clone(&row_targets);

            move || {
                while let Some(child) = saved_list.first_child() {
                    saved_list.remove(&child);
                }
                let all_servers = ServerStore::new().get_all();
                saved_frame.set_visible(!all_servers.is_empty());
                empty_label.set_visible(all_servers.is_empty());
                *row_targets.borrow_mut() = all_servers.clone();

                for srv in all_servers {
                    let row_box = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Horizontal)
                        .spacing(12)
                        .margin_top(8).margin_bottom(8).margin_start(12).margin_end(8)
                        .build();
                    row_box.append(&gtk4::Image::from_icon_name(icons::pick(icons::SERVER)));
                    let text = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    text.set_hexpand(true);
                    text.append(&gtk4::Label::builder().label(&srv.name).xalign(0.0).ellipsize(gtk4::pango::EllipsizeMode::End).build());
                    text.append(&gtk4::Label::builder()
                        .label(&format!("{} · {}@{}:{}", srv.proto.as_str().to_uppercase(), srv.user, srv.host, srv.port))
                        .xalign(0.0)
                        .ellipsize(gtk4::pango::EllipsizeMode::End)
                        .css_classes(["dim-label", "caption"])
                        .build());
                    row_box.append(&text);

                    let btn_edit = widgets::icon_button(icons::RENAME, "Profili düzenle");
                    let srv_e = srv.clone();
                    let root_e = root_ref.clone();
                    let reload_e = Rc::clone(&reload_ref);
                    btn_edit.connect_clicked(move |_| {
                        let parent = root_e.root().and_downcast::<gtk4::Window>();
                        let srv_s = srv_e.clone();
                        let reload_s = Rc::clone(&reload_e);
                        widgets::form_dialog(
                            parent.as_ref(),
                            "Sunucu Profilini Düzenle",
                            None,
                            vec![
                                Field::Text("Profil adı", srv_e.name.clone()),
                                Field::Text("Sunucu", srv_e.host.clone()),
                                Field::Text("Port", srv_e.port.to_string()),
                                Field::Text("Kullanıcı adı", srv_e.user.clone()),
                                Field::Password("Parola", srv_e.password.clone().unwrap_or_default()),
                            ],
                            "_Kaydet",
                            move |v| {
                                let updated = ServerProfile {
                                    id: srv_s.id.clone(),
                                    name: v[0].trim().to_string(),
                                    host: v[1].trim().to_string(),
                                    port: v[2].trim().parse::<u16>().unwrap_or(21),
                                    user: v[3].trim().to_string(),
                                    password: Some(v[4].clone()),
                                    proto: srv_s.proto,
                                    default_remote_dir: srv_s.default_remote_dir.clone(),
                                    default_local_dir: srv_s.default_local_dir.clone(),
                                };
                                ServerStore::new().update_server(updated);
                                if let Some(ref r) = *reload_s.borrow() {
                                    r();
                                }
                            },
                        );
                    });
                    row_box.append(&btn_edit);

                    let btn_del = widgets::icon_button(icons::DELETE, "Profili sil");
                    let srv_d = srv.clone();
                    let root_d = root_ref.clone();
                    let reload_d = Rc::clone(&reload_ref);
                    btn_del.connect_clicked(move |_| {
                        if let Some(parent) = root_d.root().and_downcast::<gtk4::Window>() {
                            let id = srv_d.id.clone();
                            let reload = Rc::clone(&reload_d);
                            widgets::confirm(
                                &parent,
                                "Sunucu profili silinsin mi?",
                                &format!("'{}' profili silinecek.", srv_d.name),
                                "Sil",
                                move || {
                                    ServerStore::new().delete_server(&id);
                                    if let Some(ref r) = *reload.borrow() {
                                        r();
                                    }
                                },
                            );
                        }
                    });
                    row_box.append(&btn_del);

                    let btn_conn = widgets::icon_button(icons::CONNECT, "Bağlan");
                    let srv_c = srv.clone();
                    let on_c = on_conn.clone();
                    btn_conn.connect_clicked(move |_| on_c(srv_c.clone()));
                    row_box.append(&btn_conn);

                    let row = gtk4::ListBoxRow::builder().child(&row_box).activatable(true).build();
                    saved_list.append(&row);
                }
            }
        };
        // Clicking / pressing Enter on a row connects to that server.
        let on_row = on_connect.clone();
        let targets_row = Rc::clone(&row_targets);
        saved_list.connect_row_activated(move |_, row| {
            let srv = targets_row.borrow().get(row.index() as usize).cloned();
            if let Some(srv) = srv {
                on_row(srv);
            }
        });

        let rc_reload: Rc<dyn Fn()> = Rc::new(reload_logic);
        *reload_saved_fn.borrow_mut() = Some(rc_reload.clone());
        rc_reload();

        // Quick-connect button
        let on_conn = on_connect.clone();
        let rc_reload_for_btn = rc_reload.clone();
        btn_primary.connect_clicked(move |_| {
            let host = host_entry.text().trim().to_string();
            if host.is_empty() {
                host_entry.add_css_class("error");
                host_entry.grab_focus();
                return;
            }
            host_entry.remove_css_class("error");

            let user = user_entry.text().trim().to_string();
            let user = if user.is_empty() { "anonymous".to_string() } else { user };
            let profile = ServerProfile {
                id: format!("{}_{}", host, user),
                name: host.clone(),
                host,
                port: port_entry.text().parse().unwrap_or(22),
                user,
                password: Some(password_entry.text().to_string()),
                proto: proto_from_index(proto_dd.selected()),
                default_remote_dir: Some("/".to_string()),
                default_local_dir: None,
            };

            if remember_cb.is_active() {
                ServerStore::new().add_server(profile.clone());
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
