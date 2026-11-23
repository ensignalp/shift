mod models;
mod server_store;
mod engine;
mod ui;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use gtk4::prelude::*;
use libadwaita::prelude::*;
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
    let app = libadwaita::Application::builder()
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
    });

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &libadwaita::Application) {
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("Shift")
        .default_width(733)
        .default_height(780)
        .width_request(580)
        .height_request(560)
        .icon_name("com.ensignsoft.Shift")
        .build();

    let toast_overlay = libadwaita::ToastOverlay::new();
    let root_vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // HeaderBar with guaranteed visible title buttons (Close, Minimize, Maximize)
    let header_bar = libadwaita::HeaderBar::builder()
        .css_classes(["flat"])
        .show_end_title_buttons(true)
        .show_start_title_buttons(true)
        .build();

    let win_title = libadwaita::WindowTitle::builder()
        .title("Shift")
        .subtitle("Sunucuya Bağlanın")
        .build();
    header_bar.set_title_widget(Some(&win_title));

    // Left Action Area
    let start_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let btn_sites = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER));
    btn_sites.add_css_class("flat");
    btn_sites.add_css_class("circular");
    btn_sites.set_tooltip_text(Some("Site Yöneticisi"));
    start_box.append(&btn_sites);

    let btn_disconnect = gtk4::Button::builder()
        .css_classes(["destructive-action", "btn-disconnect-red"])
        .valign(gtk4::Align::Center)
        .visible(false)
        .build();
    let disc_content = libadwaita::ButtonContent::builder()
        .icon_name(crate::ui::icons::pick(crate::ui::icons::CONNECT))
        .label("Bağlantıyı Kes")
        .build();
    btn_disconnect.set_child(Some(&disc_content));
    start_box.append(&btn_disconnect);
    header_bar.pack_start(&start_box);

    // Right Action Area
    let end_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

    // Transfer Queue Button with Lucide Download icon
    let btn_transfers = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::DOWNLOAD));
    btn_transfers.add_css_class("flat");
    btn_transfers.add_css_class("circular");
    btn_transfers.set_tooltip_text(Some("Dosya Aktarımları Kuyruğu"));
    end_box.append(&btn_transfers);

    // Main Hamburger Menu Button with Lucide Menu icon
    let menu_button = gtk4::MenuButton::builder()
        .icon_name(crate::ui::icons::pick(crate::ui::icons::MENU))
        .css_classes(["flat", "circular"])
        .tooltip_text("Ana Menü")
        .build();

    let menu_model = gio::Menu::new();
    menu_model.append(Some("Site Yöneticisi"), Some("app.site-manager"));
    menu_model.append(Some("Günlük Konsolunu Göster/Gizle"), Some("app.toggle-log"));
    menu_model.append(Some("Karanlık Modu Aç/Kapat"), Some("app.toggle-dark"));
    menu_model.append(Some("Shift Hakkında"), Some("app.about"));
    menu_button.set_menu_model(Some(&menu_model));
    end_box.append(&menu_button);

    header_bar.pack_end(&end_box);
    root_vbox.append(&header_bar);

    // ViewStack
    let view_stack = libadwaita::ViewStack::builder()
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
        let dialog = libadwaita::MessageDialog::builder()
            .transient_for(&win_nf)
            .heading("Yeni Klasör Oluştur")
            .body("Oluşturulacak klasörün adını girin.")
            .build();

        let entry = libadwaita::EntryRow::builder()
            .title("Klasör Adı")
            .text("Yeni Klasör")
            .build();
        let box_entry = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_entry.set_margin_top(12);
        box_entry.set_margin_bottom(12);
        box_entry.append(&entry);
        dialog.set_extra_child(Some(&box_entry));

        dialog.add_response("cancel", "İptal");
        dialog.add_response("create", "Oluştur");
        dialog.set_response_appearance("create", libadwaita::ResponseAppearance::Suggested);

        let conn = Arc::clone(&conn_nf);
        let tx = tx_nf.clone();
        let parent = parent_dir.clone();

        dialog.connect_response(None, move |_, resp| {
            if resp == "create" {
                let name = entry.text().trim().to_string();
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
        dialog.present();
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

        let dialog = libadwaita::MessageDialog::builder()
            .transient_for(&win_del)
            .heading("Öğeyi Sil?")
            .body(&format!("'{}' kalıcı olarak silinecek. Emin misiniz?", item_name))
            .build();
        dialog.add_response("cancel", "Vazgeç");
        dialog.add_response("delete", "Sil");
        dialog.set_response_appearance("delete", libadwaita::ResponseAppearance::Destructive);

        let conn = Arc::clone(&conn_del);
        let tx = tx_del.clone();
        let path = full_path.clone();
        let name = item_name.clone();
        let parent = parent_dir.clone();

        dialog.connect_response(None, move |_, resp| {
            if resp == "delete" {
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
        dialog.present();
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

        let dialog = libadwaita::MessageDialog::builder()
            .transient_for(&win_ren)
            .heading("Yeniden Adlandır")
            .body("Öğe için yeni ismi girin.")
            .build();

        let entry = libadwaita::EntryRow::builder()
            .title("Yeni İsim")
            .text(&old_name)
            .build();
        let box_entry = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_entry.set_margin_top(12);
        box_entry.set_margin_bottom(12);
        box_entry.append(&entry);
        dialog.set_extra_child(Some(&box_entry));

        dialog.add_response("cancel", "İptal");
        dialog.add_response("rename", "Değiştir");
        dialog.set_response_appearance("rename", libadwaita::ResponseAppearance::Suggested);

        let conn = Arc::clone(&conn_ren);
        let tx = tx_ren.clone();
        let parent = parent_dir.clone();
        let old_path = old_path.clone();

        dialog.connect_response(None, move |_, resp| {
            if resp == "rename" {
                let new_name = entry.text().trim().to_string();
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
        dialog.present();
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
            SiteManagerWindow::show(&w_for_sm, move |srv| {
                do_conn(srv);
            });
        },
    ));

    view_stack.add_named(&onboarding_view.container, Some("onboarding"));
    view_stack.set_visible_child_name("onboarding");

    root_vbox.append(&view_stack);
    root_vbox.append(&log_rev.container);

    toast_overlay.set_child(Some(&root_vbox));
    window.set_content(Some(&toast_overlay));

    // Receiver polling loop attached to GLib Main Context
    let log_event = Rc::clone(&log_rev);
    let exp_event = Rc::clone(&explorer_view);
    let title_event = win_title.clone();
    let btn_disc_event = btn_disconnect.clone();
    let btn_sites_event = btn_sites.clone();
    let stack_event = view_stack.clone();
    let toast_event = toast_overlay.clone();
    let win_err = window.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Log(lvl, msg) => {
                    log_event.append_log(&lvl, &msg);
                }
                AppEvent::Connected(server_str, items) => {
                    title_event.set_subtitle(&format!("● {}", server_str));
                    btn_disc_event.set_visible(true);
                    btn_sites_event.set_visible(false);
                    exp_event.set_items("/", items);
                    stack_event.set_visible_child_name("explorer");
                    toast_event.add_toast(libadwaita::Toast::new(&format!("Bağlandı: {}", server_str)));
                }
                AppEvent::DirectoryListed(path, items) => {
                    exp_event.set_items(&path, items);
                }
                AppEvent::Toast(msg) => {
                    toast_event.add_toast(libadwaita::Toast::new(&msg));
                }
                AppEvent::Error(err) => {
                    toast_event.add_toast(libadwaita::Toast::new(&err));
                    let dialog = libadwaita::MessageDialog::builder()
                        .transient_for(&win_err)
                        .modal(true)
                        .heading("Hata Bildirimi")
                        .body(&err)
                        .build();
                    dialog.add_response("copy", "Kopyala");
                    dialog.set_response_appearance("copy", libadwaita::ResponseAppearance::Suggested);
                    dialog.add_response("close", "Tamam");
                    dialog.set_default_response(Some("close"));

                    let err_copy = err.clone();
                    let t_copy = toast_event.clone();
                    dialog.connect_response(None, move |_, resp| {
                        if resp == "copy" {
                            let display = gtk4::gdk::Display::default().unwrap();
                            display.clipboard().set_text(&err_copy);
                            t_copy.add_toast(libadwaita::Toast::new("Hata mesajı panoya kopyalandı."));
                        }
                    });
                    dialog.present();
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
        SiteManagerWindow::show(&w_sm, move |srv| {
            reload();
            do_conn(srv);
        });
    });

    // Disconnect button click
    let conn_disc = Arc::clone(&active_connection);
    let view_stack_disc = view_stack.clone();
    let win_title_disc = win_title.clone();
    let btn_disc_disc = btn_disconnect.clone();
    let btn_sites_disc = btn_sites.clone();
    let log_disc = Rc::clone(&log_rev);
    let onb_reload_disc = onboarding_view.reload_saved.clone();

    btn_disconnect.connect_clicked(move |_| {
        *conn_disc.lock().unwrap() = None;
        win_title_disc.set_subtitle("Sunucuya Bağlanın");
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
        SiteManagerWindow::show(&w_sm2, move |srv| {
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

    let act_toggle_dark = gio::SimpleAction::new("toggle-dark", None);
    act_toggle_dark.connect_activate(move |_, _| {
        let mgr = libadwaita::StyleManager::default();
        if mgr.is_dark() {
            mgr.set_color_scheme(libadwaita::ColorScheme::ForceLight);
        } else {
            mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark);
        }
    });
    app.add_action(&act_toggle_dark);

    let act_about = gio::SimpleAction::new("about", None);
    let w_about = window.clone();
    act_about.connect_activate(move |_, _| {
        let dialog = libadwaita::AboutDialog::builder()
            .application_name("Shift")
            .application_icon("com.ensignsoft.Shift")
            .developer_name("Ensignsoft Mühendislik Ekibi")
            .version("1.0.0")
            .comments("Yüksek performanslı, güvenli ve modern bulut dosya aktarım istemcisi.")
            .website("https://ensignsoft.com")
            .issue_url("https://github.com/ensignsoft/shift/issues")
            .build();
        dialog.present(Some(&w_about));
    });
    app.add_action(&act_about);

    window.present();
}
