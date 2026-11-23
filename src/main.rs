mod models;
mod server_store;
mod engine;
mod ui;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use gtk4::prelude::*;
use ui::widgets::{self, Field, Toaster};
use models::{FileItem, ServerProfile};
use engine::{BackendEngine, ClientConnection};
use ui::explorer::ExplorerView;
use ui::log_revealer::LogRevealer;
use ui::onboarding::OnboardingView;
use ui::site_manager::SiteManagerWindow;

const APP_ID: &str = "com.ensignsoft.Shift";

// Global Async Action Messages from Worker Thread to Main UI Thread
pub enum AppEvent {
    Log(String, String),
    Connected(String, Vec<FileItem>),
    DirectoryListed(String, Vec<FileItem>),
    Toast(String),
    Error(String),
}

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_startup(|_| {
        let display = gtk4::gdk::Display::default().expect("Could not get default display");
        let icon_theme = gtk4::IconTheme::for_display(&display);

        // App icon only: every other icon comes from the user's icon theme.
        let gresource_bytes = glib::Bytes::from_static(include_bytes!("../data/resources.gresource"));
        if let Ok(resource) = gio::Resource::from_data(&gresource_bytes) {
            gio::resources_register(&resource);
            icon_theme.add_resource_path("/com/ensignsoft/Shift/icons");
        }

        // 3. Embedded Application CSS (No missing file errors)
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_string(include_str!("../data/style.css"));
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Desktop accent colour for GTK's built-in theme (no-op for other themes).
        ui::accent::install(&display);
    });

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Shift")
        .default_width(733)
        .default_height(780)
        .width_request(580)
        .height_request(560)
        .icon_name("com.ensignsoft.Shift")
        .build();

    let toaster = Toaster::new();
    let root_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // --- Window chrome -------------------------------------------------
    // GNOME-family desktops expect client-side header bars; KDE, XFCE, MATE,
    // Cinnamon, LXQt... draw their own title bar and buttons. In that case we
    // leave decoration to the window manager and use a plain toolbar instead,
    // so the title bar, button layout and button theme are the desktop's own.
    let csd = use_client_side_decorations();

    let win_title = gtk4::Label::builder().label("Shift").css_classes(["heading"]).build();
    let win_subtitle = gtk4::Label::builder()
        .label("Sunucuya bağlı değil")
        .css_classes(["caption", "dim-label"])
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .build();

    let start_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let btn_sites = widgets::icon_button(ui::icons::SERVER, "Site Yöneticisi");
    start_box.append(&btn_sites);

    let btn_disconnect = gtk4::Button::builder()
        .label("Bağlantıyı Kes")
        .tooltip_text("Sunucu bağlantısını sonlandır")
        .valign(gtk4::Align::Center)
        .visible(false)
        .build();
    start_box.append(&btn_disconnect);

    let end_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let btn_transfers = widgets::icon_button(ui::icons::TRANSFERS, "Dosya Aktarımları Kuyruğu");
    end_box.append(&btn_transfers);

    let menu_button = gtk4::MenuButton::builder()
        .icon_name(ui::icons::pick(ui::icons::MENU))
        .tooltip_text("Ana Menü")
        .valign(gtk4::Align::Center)
        .build();
    menu_button.add_css_class("flat");

    let menu_model = gio::Menu::new();
    let sec_main = gio::Menu::new();
    sec_main.append(Some("Site Yöneticisi"), Some("app.site-manager"));
    sec_main.append(Some("Günlük Konsolunu Göster/Gizle"), Some("app.toggle-log"));
    menu_model.append_section(None, &sec_main);
    let sec_theme = gio::Menu::new();
    sec_theme.append(Some("Sistem Teması"), Some("app.color-scheme::system"));
    sec_theme.append(Some("Açık"), Some("app.color-scheme::light"));
    sec_theme.append(Some("Koyu"), Some("app.color-scheme::dark"));
    menu_model.append_section(Some("Görünüm"), &sec_theme);
    let sec_about = gio::Menu::new();
    sec_about.append(Some("Shift Hakkında"), Some("app.about"));
    menu_model.append_section(None, &sec_about);
    menu_button.set_menu_model(Some(&menu_model));
    end_box.append(&menu_button);

    if csd {
        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        title_box.set_valign(gtk4::Align::Center);
        title_box.append(&win_title);
        title_box.append(&win_subtitle);
        let header_bar = gtk4::HeaderBar::new();
        header_bar.pack_start(&start_box);
        header_bar.set_title_widget(Some(&title_box));
        header_bar.pack_end(&end_box);
        window.set_titlebar(Some(&header_bar));
    } else {
        // The WM title bar already shows the window title.
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        toolbar.add_css_class("app-toolbar");
        win_subtitle.set_hexpand(true);
        win_subtitle.set_xalign(0.0);
        toolbar.append(&start_box);
        toolbar.append(&win_subtitle);
        toolbar.append(&end_box);
        root_vbox.append(&toolbar);
        root_vbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    }

    // ViewStack
    let view_stack = gtk4::Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .build();

    let active_connection: Arc<Mutex<Option<ClientConnection>>> = Arc::new(Mutex::new(None));
    let log_rev = Rc::new(LogRevealer::new());

    // High performance async channel for thread-safe cross-thread UI communication
    let (tx, rx) = async_channel::unbounded::<AppEvent>();

    // Explorer View Action Handlers
    let conn_nav = Arc::clone(&active_connection);
    let tx_nav = tx.clone();
    let on_navigate = move |path: String| {
        let conn = Arc::clone(&conn_nav);
        let tx = tx_nav.clone();
        std::thread::spawn(move || {
            let mut lock = conn.lock().unwrap();
            if let Some(ref mut c) = *lock {
                let _ = tx.send_blocking(AppEvent::Log("command".into(), format!("CWD {}", path)));
                match BackendEngine::list_directory(c, &path) {
                    Ok(items) => {
                        let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("226 Listing complete ({} öğe)", items.len())));
                        let _ = tx.send_blocking(AppEvent::DirectoryListed(path, items));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                        let _ = tx.send_blocking(AppEvent::Error(e));
                    }
                }
            }
        });
    };

    let conn_up = Arc::clone(&active_connection);
    let tx_up = tx.clone();
    let win_up = window.clone();
    let on_upload = move |current_remote_dir: String| {
        let dialog = gtk4::FileDialog::builder()
            .title("Yüklenecek Dosyayı Seçin")
            .modal(true)
            .build();
        let conn = Arc::clone(&conn_up);
        let tx = tx_up.clone();
        let remote_dir = current_remote_dir.clone();
        dialog.open(Some(&win_up), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let remote_file_path = if remote_dir == "/" {
                        format!("/{}", file_name)
                    } else {
                        format!("{}/{}", remote_dir.trim_end_matches('/'), file_name)
                    };
                    let tx = tx.clone();
                    let conn = Arc::clone(&conn);
                    let r_dir = remote_dir.clone();
                    let _ = tx.send_blocking(AppEvent::Log("status".into(), format!("Yükleniyor: {} -> {}", path.display(), remote_file_path)));
                    std::thread::spawn(move || {
                        let mut lock = conn.lock().unwrap();
                        if let Some(ref mut c) = *lock {
                            match BackendEngine::upload_file(c, &path, &remote_file_path) {
                                Ok(()) => {
                                    let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("226 Yükleme tamamlandı: {}", file_name)));
                                    let _ = tx.send_blocking(AppEvent::Toast(format!("Yüklendi: {}", file_name)));
                                    if let Ok(items) = BackendEngine::list_directory(c, &r_dir) {
                                        let _ = tx.send_blocking(AppEvent::DirectoryListed(r_dir, items));
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                                    let _ = tx.send_blocking(AppEvent::Error(e));
                                }
                            }
                        }
                    });
                }
            }
        });
    };

    let conn_dl = Arc::clone(&active_connection);
    let tx_dl = tx.clone();
    let on_download = move |item: FileItem| {
        let download_dir = dirs::download_dir().unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            p.push("Downloads");
            p
        });
        let _ = std::fs::create_dir_all(&download_dir);
        let local_path = download_dir.join(&item.name);
        let tx = tx_dl.clone();
        let conn = Arc::clone(&conn_dl);
        let item_name = item.name.clone();
        let item_path = item.path.clone();
        let local_disp = local_path.display().to_string();

        let _ = tx.send_blocking(AppEvent::Log("status".into(), format!("İndiriliyor: {} -> {}", item_path, local_disp)));

        std::thread::spawn(move || {
            let mut lock = conn.lock().unwrap();
            if let Some(ref mut c) = *lock {
                match BackendEngine::download_file(c, &item_path, &local_path) {
                    Ok(()) => {
                        let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("226 İndirme tamamlandı: {}", item_name)));
                        let _ = tx.send_blocking(AppEvent::Toast(format!("İndirildi: {} (İndirilenler klasörüne kaydedildi)", item_name)));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                        let _ = tx.send_blocking(AppEvent::Error(e));
                    }
                }
            }
        });
    };

    let conn_nf = Arc::clone(&active_connection);
    let tx_nf = tx.clone();
    let win_nf = window.clone();
    let on_new_folder = move |parent_dir: String| {
        let conn = Arc::clone(&conn_nf);
        let tx = tx_nf.clone();
        let parent = parent_dir.clone();

        widgets::form_dialog(Some(win_nf.upcast_ref()), "Yeni Klasör", Some("Oluşturulacak klasörün adını girin."),
            vec![Field::Text("Klasör adı", "Yeni Klasör".into())], "_Oluştur", move |vals| {
            {
                let name = vals[0].trim().to_string();
                if name.is_empty() { return; }
                let conn = Arc::clone(&conn);
                let tx = tx.clone();
                let parent = parent.clone();

                let _ = tx.send_blocking(AppEvent::Log("command".into(), format!("MKD {}", name)));
                std::thread::spawn(move || {
                    let mut lock = conn.lock().unwrap();
                    if let Some(ref mut c) = *lock {
                        match BackendEngine::create_directory(c, &parent, &name) {
                            Ok(()) => {
                                let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("257 \"{}\" oluşturuldu.", name)));
                                let _ = tx.send_blocking(AppEvent::Toast(format!("Klasör oluşturuldu: {}", name)));
                                if let Ok(items) = BackendEngine::list_directory(c, &parent) {
                                    let _ = tx.send_blocking(AppEvent::DirectoryListed(parent, items));
                                }
                            }
                            Err(e) => {
                                let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                                let _ = tx.send_blocking(AppEvent::Error(e));
                            }
                        }
                    }
                });
            }
        });
    };

    let conn_del = Arc::clone(&active_connection);
    let tx_del = tx.clone();
    let win_del = window.clone();
    let on_delete = move |full_path: String, is_dir: bool| {
        let item_name = std::path::Path::new(&full_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| full_path.clone());
        let parent_dir = std::path::Path::new(&full_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());

        let conn = Arc::clone(&conn_del);
        let tx = tx_del.clone();
        let path = full_path.clone();
        let name = item_name.clone();
        let parent = parent_dir.clone();

        widgets::confirm(&win_del, "Öğe silinsin mi?", &format!("'{}' kalıcı olarak silinecek.", item_name), "Sil", move || {
            {
                let conn = Arc::clone(&conn);
                let tx = tx.clone();
                let path = path.clone();
                let name = name.clone();
                let parent = parent.clone();

                let _ = tx.send_blocking(AppEvent::Log("command".into(), format!("DELE {}", path)));
                std::thread::spawn(move || {
                    let mut lock = conn.lock().unwrap();
                    if let Some(ref mut c) = *lock {
                        match BackendEngine::delete_item(c, &path, is_dir) {
                            Ok(()) => {
                                let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("250 \"{}\" silindi.", name)));
                                let _ = tx.send_blocking(AppEvent::Toast(format!("Silindi: {}", name)));
                                if let Ok(items) = BackendEngine::list_directory(c, &parent) {
                                    let _ = tx.send_blocking(AppEvent::DirectoryListed(parent, items));
                                }
                            }
                            Err(e) => {
                                let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                                let _ = tx.send_blocking(AppEvent::Error(e));
                            }
                        }
                    }
                });
            }
        });
    };

    let conn_ren = Arc::clone(&active_connection);
    let tx_ren = tx.clone();
    let win_ren = window.clone();
    let on_rename = move |old_path: String, _new_path: String| {
        let old_name = std::path::Path::new(&old_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.clone());
        let parent_dir = std::path::Path::new(&old_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());

        let conn = Arc::clone(&conn_ren);
        let tx = tx_ren.clone();
        let parent = parent_dir.clone();
        let old_path = old_path.clone();

        let old_name_init = old_name.clone();
        widgets::form_dialog(Some(win_ren.upcast_ref()), "Yeniden Adlandır", None,
            vec![Field::Text("Yeni ad", old_name_init)], "_Yeniden Adlandır", move |vals| {
            {
                let new_name = vals[0].trim().to_string();
                if new_name.is_empty() || new_name == old_name { return; }
                let new_full_path = if parent == "/" {
                    format!("/{}", new_name)
                } else {
                    format!("{}/{}", parent.trim_end_matches('/'), new_name)
                };

                let conn = Arc::clone(&conn);
                let tx = tx.clone();
                let parent = parent.clone();
                let old_path = old_path.clone();

                let _ = tx.send_blocking(AppEvent::Log("command".into(), format!("RNFR {} -> RNTO {}", old_path, new_full_path)));
                std::thread::spawn(move || {
                    let mut lock = conn.lock().unwrap();
                    if let Some(ref mut c) = *lock {
                        match BackendEngine::rename_item(c, &old_path, &new_full_path) {
                            Ok(()) => {
                                let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("250 Yeniden adlandırıldı: {}", new_name)));
                                let _ = tx.send_blocking(AppEvent::Toast(format!("Yeniden adlandırıldı: {}", new_name)));
                                if let Ok(items) = BackendEngine::list_directory(c, &parent) {
                                    let _ = tx.send_blocking(AppEvent::DirectoryListed(parent, items));
                                }
                            }
                            Err(e) => {
                                let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                                let _ = tx.send_blocking(AppEvent::Error(e));
                            }
                        }
                    }
                });
            }
        });
    };

    let explorer_view = Rc::new(ExplorerView::new(
        on_navigate,
        on_upload,
        on_download,
        on_new_folder,
        on_delete,
        on_rename,
    ));

    view_stack.add_named(&explorer_view.container, Some("explorer"));

    // Global Connect Logic (Shared by Onboarding and Site Manager)
    let do_connect = {
        let conn_shared = Arc::clone(&active_connection);
        let tx = tx.clone();
        move |srv: ServerProfile| {
            let conn_shared = Arc::clone(&conn_shared);
            let tx = tx.clone();

            let _ = tx.send_blocking(AppEvent::Log(
                "status".into(),
                format!(
                    "Bağlanıyor: {}@{}:{} ({})",
                    srv.user,
                    srv.host,
                    srv.port,
                    srv.proto.as_str().to_uppercase()
                ),
            ));

            std::thread::spawn(move || {
                let pass = srv.password.as_deref().unwrap_or("");
                match BackendEngine::connect(srv.proto, &srv.host, srv.port, &srv.user, pass) {
                    Ok(mut conn) => {
                        let _ = tx.send_blocking(AppEvent::Log("response".into(), "230 Giriş başarılı.".into()));
                        let start_dir = srv.default_remote_dir.as_deref().unwrap_or("/");
                        let start_dir = if start_dir.is_empty() { "/" } else { start_dir };
                        let items = BackendEngine::list_directory(&mut conn, start_dir).unwrap_or_default();
                        *conn_shared.lock().unwrap() = Some(conn);
                        let _ = tx.send_blocking(AppEvent::Connected(
                            format!("{}@{}:{}", srv.user, srv.host, srv.port),
                            items,
                        ));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Log("error".into(), e.clone()));
                        let _ = tx.send_blocking(AppEvent::Error(format!("Bağlantı Hatası: {}", e)));
                    }
                }
            });
        }
    };

    // Onboarding View
    let w_for_sm = window.clone();
    let do_connect_for_sm = do_connect.clone();
    let onboarding_view = Rc::new(OnboardingView::new(
        do_connect.clone(),
        move || {
            let do_conn = do_connect_for_sm.clone();
            SiteManagerWindow::show(w_for_sm.upcast_ref(), move |srv| {
                do_conn(srv);
            });
        },
    ));

    view_stack.add_named(&onboarding_view.container, Some("onboarding"));
    view_stack.set_visible_child_name("onboarding");

    root_vbox.append(&view_stack);
    // The hidden log panel must not compete with the main view for space.
    log_rev.container.set_vexpand(false);
    root_vbox.append(&log_rev.container);

    toaster.overlay.set_child(Some(&root_vbox));
    window.set_child(Some(&toaster.overlay));

    // Receiver polling loop attached to GLib Main Context
    let log_event = Rc::clone(&log_rev);
    let exp_event = Rc::clone(&explorer_view);
    let title_event = win_subtitle.clone();
    let win_title_event = window.clone();
    let btn_disc_event = btn_disconnect.clone();
    let btn_sites_event = btn_sites.clone();
    let stack_event = view_stack.clone();
    let toast_event = toaster.clone();
    let win_err = window.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Log(lvl, msg) => {
                    log_event.append_log(&lvl, &msg);
                }
                AppEvent::Connected(server_str, items) => {
                    title_event.set_label(&server_str);
                    win_title_event.set_title(Some(&format!("{} — Shift", server_str)));
                    gtk4::prelude::GtkWindowExt::set_focus(&win_title_event, None::<&gtk4::Widget>);
                    btn_disc_event.set_visible(true);
                    btn_sites_event.set_visible(false);
                    exp_event.set_items("/", items);
                    stack_event.set_visible_child_name("explorer");
                    toast_event.show(&format!("Bağlandı: {}", server_str));
                }
                AppEvent::DirectoryListed(path, items) => {
                    exp_event.set_items(&path, items);
                }
                AppEvent::Toast(msg) => {
                    toast_event.show(&msg);
                }
                AppEvent::Error(err) => {
                    let err_copy = err.clone();
                    let t_copy = toast_event.clone();
                    widgets::error(&win_err, &err, move || {
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&err_copy);
                        }
                        t_copy.show("Hata mesajı panoya kopyalandı.");
                    });
                }
            }
        }
        glib::ControlFlow::Continue
    });

    // Site Manager button click
    let w_sm = window.clone();
    let do_conn_sm = do_connect.clone();
    let onb_reload_sm = onboarding_view.reload_saved.clone();
    btn_sites.connect_clicked(move |_| {
        let do_conn = do_conn_sm.clone();
        let reload = onb_reload_sm.clone();
        SiteManagerWindow::show(w_sm.upcast_ref(), move |srv| {
            reload();
            do_conn(srv);
        });
    });

    // Disconnect button click
    let conn_disc = Arc::clone(&active_connection);
    let view_stack_disc = view_stack.clone();
    let win_title_disc = win_subtitle.clone();
    let win_disc = window.clone();
    let btn_disc_disc = btn_disconnect.clone();
    let btn_sites_disc = btn_sites.clone();
    let log_disc = Rc::clone(&log_rev);
    let onb_reload_disc = onboarding_view.reload_saved.clone();

    btn_disconnect.connect_clicked(move |_| {
        *conn_disc.lock().unwrap() = None;
        win_title_disc.set_label("Sunucuya bağlı değil");
        win_disc.set_title(Some("Shift"));
        btn_disc_disc.set_visible(false);
        btn_sites_disc.set_visible(true);
        onb_reload_disc();
        view_stack_disc.set_visible_child_name("onboarding");
        log_disc.append_log("status", "Bağlantı sonlandırıldı.");
    });

    // Register actions for Hamburger Menu
    let act_site_mgr = gio::SimpleAction::new("site-manager", None);
    let w_sm2 = window.clone();
    let do_conn_sm2 = do_connect.clone();
    let onb_reload_sm2 = onboarding_view.reload_saved.clone();
    act_site_mgr.connect_activate(move |_, _| {
        let do_conn = do_conn_sm2.clone();
        let reload = onb_reload_sm2.clone();
        SiteManagerWindow::show(w_sm2.upcast_ref(), move |srv| {
            reload();
            do_conn(srv);
        });
    });
    app.add_action(&act_site_mgr);

    let act_toggle_log = gio::SimpleAction::new("toggle-log", None);
    let log_rev_toggle = Rc::clone(&log_rev);
    act_toggle_log.connect_activate(move |_, _| {
        let is_open = log_rev_toggle.revealer.reveals_child();
        log_rev_toggle.revealer.set_reveal_child(!is_open);
    });
    app.add_action(&act_toggle_log);

    // Appearance: follow the system by default; explicit light/dark on request.
    let act_scheme = gio::SimpleAction::new_stateful(
        "color-scheme",
        Some(glib::VariantTy::STRING),
        &"system".to_variant(),
    );
    if let Some(settings) = gtk4::Settings::default() {
        let original = (settings.gtk_theme_name(), settings.is_gtk_application_prefer_dark_theme());
        let mode = Rc::new(std::cell::RefCell::new(String::from("system")));
        // GTK's built-in theme does not apply the desktop's dark preference by
        // itself (libadwaita used to); follow it while in "system" mode.
        follow_system_scheme(&settings, original.1);
        let mode_n = Rc::clone(&mode);
        let orig_n = original.1;
        settings.connect_notify_local(Some("gtk-interface-color-scheme"), move |s, _| {
            if mode_n.borrow().as_str() == "system" {
                follow_system_scheme(s, orig_n);
            }
        });
        act_scheme.connect_activate(move |act, param| {
            let m = param.and_then(|p| p.str()).unwrap_or("system").to_string();
            *mode.borrow_mut() = m.clone();
            apply_color_scheme(&settings, &m, &original);
            act.set_state(&m.to_variant());
        });
    }
    app.add_action(&act_scheme);

    let act_about = gio::SimpleAction::new("about", None);
    let w_about = window.clone();
    act_about.connect_activate(move |_, _| {
        let dialog = gtk4::AboutDialog::builder()
            .transient_for(&w_about)
            .modal(true)
            .program_name("Shift")
            .logo_icon_name("com.ensignsoft.Shift")
            .authors(["Ensignsoft Mühendislik Ekibi"])
            .version(env!("CARGO_PKG_VERSION"))
            .comments("Yüksek performanslı, güvenli ve modern bulut dosya aktarım istemcisi.")
            .website("https://ensignsoft.com")
            .build();
        dialog.present();
    });
    app.add_action(&act_about);

    window.present();

    // Debug aid: SHIFT_AUTOCONNECT=<n> connects to the n-th saved site at startup.
    if let Some(idx) = std::env::var("SHIFT_AUTOCONNECT").ok().and_then(|v| v.parse::<usize>().ok()) {
        if let Some(srv) = server_store::ServerStore::new().get_all().into_iter().nth(idx) {
            do_connect(srv);
        }
    }

    if let Ok(path) = std::env::var("SHIFT_SNAPSHOT") {
        debug_snapshot(&window, path);
    }
}

/// GNOME-family shells have no server-side decorations and expect header
/// bars; everywhere else the window manager draws the title bar.
/// Override with SHIFT_CSD=1 / SHIFT_CSD=0.
fn use_client_side_decorations() -> bool {
    if let Ok(v) = std::env::var("SHIFT_CSD") {
        return v == "1" || v.eq_ignore_ascii_case("true");
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    desktop
        .split(':')
        .any(|d| matches!(d, "gnome" | "gnome-classic" | "gnome-flashback" | "pantheon" | "budgie" | "budgie-desktop" | "unity" | "cosmic" | "phosh"))
}

/// Set an enum GObject property by its nick, if the property exists in the
/// running GTK (keeps us compatible with GTK < 4.20).
fn set_enum_by_nick(obj: &impl IsA<glib::Object>, prop: &str, nick: &str) {
    if let Some(pspec) = obj.find_property(prop) {
        if let Some(class) = glib::EnumClass::with_type(pspec.value_type()) {
            if let Some(value) = class.to_value_by_nick(nick) {
                obj.set_property_from_value(prop, &value);
            }
        }
    }
}

fn enum_nick(obj: &impl IsA<glib::Object>, prop: &str) -> Option<String> {
    obj.find_property(prop)?;
    let value = obj.property_value(prop);
    glib::EnumValue::from_value(&value).map(|(_, v)| v.nick().to_string())
}

/// Use the theme's dark variant when the desktop prefers dark (GTK >= 4.20
/// exposes the portal value as gtk-interface-color-scheme). Otherwise keep
/// whatever the desktop configured (e.g. KDE writes prefer-dark/Breeze-Dark).
fn follow_system_scheme(settings: &gtk4::Settings, orig_dark: bool) {
    match enum_nick(settings, "gtk-interface-color-scheme").as_deref() {
        Some("dark") => settings.set_gtk_application_prefer_dark_theme(true),
        Some("light") => settings.set_gtk_application_prefer_dark_theme(false),
        _ => settings.set_gtk_application_prefer_dark_theme(orig_dark),
    }
}

fn apply_color_scheme(settings: &gtk4::Settings, mode: &str, original: &(Option<glib::GString>, bool)) {
    // GTK >= 4.20 understands gtk-interface-color-scheme (default/light/dark).
    let has_scheme = settings.find_property("gtk-interface-color-scheme").is_some();
    let (orig_theme, orig_dark) = original;
    let orig_theme = orig_theme.as_ref().map(|t| t.to_string()).unwrap_or_default();
    let base_theme = orig_theme
        .strip_suffix("-dark")
        .or_else(|| orig_theme.strip_suffix("-Dark"))
        .unwrap_or(&orig_theme)
        .to_string();
    match mode {
        "dark" => {
            if has_scheme { set_enum_by_nick(settings, "gtk-interface-color-scheme", "dark"); }
            settings.set_gtk_application_prefer_dark_theme(true);
        }
        "light" => {
            if has_scheme { set_enum_by_nick(settings, "gtk-interface-color-scheme", "light"); }
            settings.set_gtk_application_prefer_dark_theme(false);
            if base_theme != orig_theme { settings.set_gtk_theme_name(Some(&base_theme)); }
        }
        _ => {
            if has_scheme { set_enum_by_nick(settings, "gtk-interface-color-scheme", "default"); }
            settings.set_gtk_theme_name(Some(&orig_theme));
            follow_system_scheme(settings, *orig_dark);
        }
    }
}

/// Debug aid: SHIFT_SNAPSHOT=/tmp/x.png renders the window to a PNG and quits.
fn debug_snapshot(window: &gtk4::ApplicationWindow, path: String) {
    let window = window.clone();
    // SHIFT_ACTION=<app action> (e.g. site-manager, about) is triggered first;
    // every visible toplevel is then saved (<path>, <path>.1.png, ...).
    if let Ok(action) = std::env::var("SHIFT_ACTION") {
        let app = window.application();
        glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
            if let Some(app) = app {
                app.activate_action(&action, None);
            }
        });
    }
    let delay = std::env::var("SHIFT_SNAPSHOT_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
        if std::env::var("SHIFT_DUMP_TREE").is_ok() {
            fn dump(w: &gtk4::Widget, depth: usize, root: &gtk4::Widget) {
                if let Some(b) = w.compute_bounds(root) {
                    eprintln!("{}{} vis={} y={:.0} h={:.0} w={:.0}", "  ".repeat(depth), w.type_().name(), w.is_visible(), b.y(), b.height(), b.width());
                }
                let mut c = w.first_child();
                while let Some(ch) = c { dump(&ch, depth + 1, root); c = ch.next_sibling(); }
            }
            dump(window.upcast_ref(), 0, window.upcast_ref());
        }
        let mut wins: Vec<gtk4::Window> = vec![window.clone().upcast()];
        let tops = gtk4::Window::list_toplevels();
        for t in tops {
            if let Ok(w) = t.downcast::<gtk4::Window>() {
                if w.is_visible() && w != *window.upcast_ref::<gtk4::Window>() {
                    wins.push(w);
                }
            }
        }
        for (i, w) in wins.iter().enumerate() {
            let paintable = gtk4::WidgetPaintable::new(Some(w));
            let snap = gtk4::Snapshot::new();
            paintable.snapshot(&snap, w.width() as f64, w.height() as f64);
            if let (Some(node), Some(renderer)) = (snap.to_node(), w.native().and_then(|n| n.renderer())) {
                let texture = renderer.render_texture(&node, None);
                let out = if i == 0 { path.clone() } else { format!("{}.{}.png", path, i) };
                let _ = texture.save_to_png(&out);
            }
        }
        if let Some(app) = window.application() {
            app.quit();
        }
    });
}
