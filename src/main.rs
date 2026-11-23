mod models;
mod server_store;
mod engine;
mod ui;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use gtk4::prelude::*;
use std::cell::RefCell;
use ui::widgets::{self, Field, Toaster};
use models::{FileItem, ServerProfile};
use engine::{BackendEngine, ClientConnection};
use ui::explorer::ExplorerView;
use ui::hosts::HostsPage;
use ui::icons;
use ui::log_page::LogPage;
use ui::theme::Theme;
use ui::transfers::{TransferState, TransfersPage};

const APP_ID: &str = "com.ensignsoft.Shift";

// Global Async Action Messages from Worker Thread to Main UI Thread
pub enum AppEvent {
    Log(String, String),
    Connected(String, Vec<FileItem>),
    DirectoryListed(String, Vec<FileItem>),
    Toast(String),
    Error(String),
    /// id, file name, is_upload, state
    Transfer(u64, String, bool, TransferState),
}

static NEXT_TRANSFER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_transfer_id() -> u64 {
    NEXT_TRANSFER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
    static THEME: RefCell<Option<Rc<Theme>>> = const { RefCell::new(None) };
}

fn main() -> glib::ExitCode {
    // Shift renders its own design on top of GTK's built-in theme. A forced
    // GTK_THEME (e.g. "Breeze:dark") would replace that base, so keep it only
    // as a light/dark hint for ui::theme.
    if let Ok(t) = std::env::var("GTK_THEME") {
        if !t.to_lowercase().starts_with("adwaita") {
            std::env::set_var("SHIFT_HOST_GTK_THEME", &t);
            std::env::remove_var("GTK_THEME");
        }
    }
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

        // Small structural helpers (typography); colours live in ui::theme.
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_string(include_str!("../data/style.css"));
        gtk4::style_context_add_provider_for_display(&display, &css_provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION - 1);

        let theme = Theme::install(&display);
        THEME.with(|t| *t.borrow_mut() = Some(theme));
    });

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Shift")
        .default_width(1080)
        .default_height(700)
        .width_request(720)
        .height_request(480)
        .icon_name("com.ensignsoft.Shift")
        .css_classes(["shift"])
        .build();

    let toaster = Toaster::new();

    // GNOME-family desktops expect a client-side header bar; KDE, XFCE, MATE,
    // Cinnamon... draw their own title bar and buttons (with their own layout
    // and theme). Override with SHIFT_CSD=1/0.
    if use_client_side_decorations() {
        let header_bar = gtk4::HeaderBar::new();
        header_bar.set_title_widget(Some(&gtk4::Label::builder().label("Shift").css_classes(["heading"]).build()));
        window.set_titlebar(Some(&header_bar));
    }

    let page_stack = gtk4::Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .transition_type(gtk4::StackTransitionType::Crossfade)
        .transition_duration(120)
        .build();

    let active_connection: Arc<Mutex<Option<ClientConnection>>> = Arc::new(Mutex::new(None));
    let log_rev = Rc::new(LogPage::new());
    let transfers = Rc::new(TransfersPage::new());

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
                    let tid = next_transfer_id();
                    let _ = tx.send_blocking(AppEvent::Transfer(tid, file_name.clone(), true, TransferState::Running));
                    std::thread::spawn(move || {
                        let mut lock = conn.lock().unwrap();
                        if let Some(ref mut c) = *lock {
                            match BackendEngine::upload_file(c, &path, &remote_file_path) {
                                Ok(()) => {
                                    let _ = tx.send_blocking(AppEvent::Transfer(tid, file_name.clone(), true, TransferState::Done));
                                    let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("226 Yükleme tamamlandı: {}", file_name)));
                                    let _ = tx.send_blocking(AppEvent::Toast(format!("Yüklendi: {}", file_name)));
                                    if let Ok(items) = BackendEngine::list_directory(c, &r_dir) {
                                        let _ = tx.send_blocking(AppEvent::DirectoryListed(r_dir, items));
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send_blocking(AppEvent::Transfer(tid, file_name.clone(), true, TransferState::Failed(e.clone())));
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
        let tid = next_transfer_id();
        let _ = tx.send_blocking(AppEvent::Transfer(tid, item_name.clone(), false, TransferState::Running));

        std::thread::spawn(move || {
            let mut lock = conn.lock().unwrap();
            if let Some(ref mut c) = *lock {
                match BackendEngine::download_file(c, &item_path, &local_path) {
                    Ok(()) => {
                        let _ = tx.send_blocking(AppEvent::Transfer(tid, item_name.clone(), false, TransferState::Done));
                        let _ = tx.send_blocking(AppEvent::Log("response".into(), format!("226 İndirme tamamlandı: {}", item_name)));
                        let _ = tx.send_blocking(AppEvent::Toast(format!("İndirildi: {} (İndirilenler klasörüne kaydedildi)", item_name)));
                    }
                    Err(e) => {
                        let _ = tx.send_blocking(AppEvent::Transfer(tid, item_name.clone(), false, TransferState::Failed(e.clone())));
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

    // ---------------------------------------------------------------- pages
    let hosts = Rc::new(HostsPage::new(do_connect.clone()));

    let files_page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_top(24).margin_bottom(16).margin_start(28).margin_end(28)
        .build();
    let files_head = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    files_head.append(&gtk4::Label::builder().label("Dosyalar").css_classes(["page-title"]).build());
    let files_dot = gtk4::Box::builder().css_classes(["status-dot", "online"]).valign(gtk4::Align::Center).build();
    files_head.append(&files_dot);
    let files_server = gtk4::Label::builder().css_classes(["dim"]).hexpand(true).xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle).build();
    files_head.append(&files_server);
    let btn_disconnect = gtk4::Button::builder().label("Bağlantıyı kes").css_classes(["danger"]).build();
    files_head.append(&btn_disconnect);
    files_page.append(&files_head);
    files_page.append(&explorer_view.container);

    page_stack.add_named(&hosts.container, Some("hosts"));
    page_stack.add_named(&files_page, Some("files"));
    page_stack.add_named(&transfers.container, Some("transfers"));
    page_stack.add_named(&log_rev.container, Some("log"));

    // -------------------------------------------------------------- sidebar
    let sidebar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .width_request(220)
        .hexpand(false)
        .css_classes(["sidebar"])
        .build();
    let brand = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(10)
        .margin_start(6).margin_top(2).margin_bottom(14).build();
    let logo = gtk4::Image::from_icon_name("com.ensignsoft.Shift");
    logo.set_pixel_size(28);
    brand.append(&logo);
    brand.append(&gtk4::Label::builder().label("Shift").css_classes(["brand"]).build());
    sidebar.append(&brand);

    let nav = |icon: &[&'static str], label: &str, page: &str| -> gtk4::ToggleButton {
        let b = gtk4::ToggleButton::builder().css_classes(["nav-item"]).build();
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        row.append(&gtk4::Image::from_icon_name(icons::pick(icon)));
        row.append(&gtk4::Label::builder().label(label).xalign(0.0).hexpand(true).build());
        b.set_child(Some(&row));
        let stack = page_stack.clone();
        let name = page.to_string();
        b.connect_toggled(move |b| {
            if b.is_active() {
                stack.set_visible_child_name(&name);
            }
        });
        b
    };
    let nav_hosts = nav(icons::SERVER, "Sunucular", "hosts");
    let nav_files = nav(icons::FOLDER, "Dosyalar", "files");
    let nav_transfers = nav(icons::TRANSFERS, "Aktarımlar", "transfers");
    let nav_log = nav(icons::TERMINAL, "Günlük", "log");
    for b in [&nav_files, &nav_transfers, &nav_log] {
        b.set_group(Some(&nav_hosts));
    }
    nav_hosts.set_active(true);
    nav_files.set_sensitive(false);
    nav_files.set_tooltip_text(Some("Önce bir sunucuya bağlanın"));
    for b in [&nav_hosts, &nav_files, &nav_transfers, &nav_log] {
        sidebar.append(b);
    }

    let sb_spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    sb_spacer.set_vexpand(true);
    sidebar.append(&sb_spacer);

    // Connection status card (Kota "Canlı" card)
    let conn_card = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).spacing(4)
        .css_classes(["card", "padded"]).margin_bottom(8).build();
    let conn_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let conn_dot = gtk4::Box::builder().css_classes(["status-dot"]).valign(gtk4::Align::Center).build();
    conn_row.append(&conn_dot);
    let conn_title = gtk4::Label::builder().label("Bağlı değil").xalign(0.0).css_classes(["heading"])
        .ellipsize(gtk4::pango::EllipsizeMode::Middle).build();
    conn_row.append(&conn_title);
    conn_card.append(&conn_row);
    conn_card.append(&log_rev.last_line);
    sidebar.append(&conn_card);

    // Settings menu
    let menu_model = gio::Menu::new();
    let sec_theme = gio::Menu::new();
    sec_theme.append(Some("Sistemi izle"), Some("app.color-scheme::system"));
    sec_theme.append(Some("Açık"), Some("app.color-scheme::light"));
    sec_theme.append(Some("Koyu"), Some("app.color-scheme::dark"));
    menu_model.append_section(Some("Görünüm"), &sec_theme);
    let sec_about = gio::Menu::new();
    sec_about.append(Some("Shift Hakkında"), Some("app.about"));
    menu_model.append_section(None, &sec_about);
    let settings_btn = gtk4::MenuButton::builder()
        .menu_model(&menu_model)
        .css_classes(["nav-item"])
        .direction(gtk4::ArrowType::Up)
        .build();
    let srow = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    srow.append(&gtk4::Image::from_icon_name(icons::pick(icons::SETTINGS)));
    srow.append(&gtk4::Label::builder().label("Ayarlar").xalign(0.0).hexpand(true).build());
    settings_btn.set_child(Some(&srow));
    sidebar.append(&settings_btn);

    // --------------------------------------------------------------- layout
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    root.append(&sidebar);
    root.append(&page_stack);
    toaster.overlay.set_child(Some(&root));
    window.set_child(Some(&toaster.overlay));

    // ---------------------------------------------------------- event pump
    let log_event = Rc::clone(&log_rev);
    let exp_event = Rc::clone(&explorer_view);
    let transfers_event = Rc::clone(&transfers);
    let toast_event = toaster.clone();
    let win_event = window.clone();
    let (files_server_ev, conn_title_ev, conn_dot_ev) = (files_server.clone(), conn_title.clone(), conn_dot.clone());
    let (nav_files_ev, nav_transfers_ev) = (nav_files.clone(), nav_transfers.clone());

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Log(lvl, msg) => log_event.append_log(&lvl, &msg),
                AppEvent::Connected(server_str, items) => {
                    files_server_ev.set_label(&server_str);
                    conn_title_ev.set_label(&server_str);
                    conn_dot_ev.add_css_class("online");
                    win_event.set_title(Some(&format!("{} — Shift", server_str)));
                    exp_event.set_items("/", items);
                    nav_files_ev.set_sensitive(true);
                    nav_files_ev.set_tooltip_text(None);
                    nav_files_ev.set_active(true);
                    gtk4::prelude::GtkWindowExt::set_focus(&win_event, None::<&gtk4::Widget>);
                    toast_event.show(&format!("Bağlandı: {}", server_str));
                }
                AppEvent::DirectoryListed(path, items) => exp_event.set_items(&path, items),
                AppEvent::Toast(msg) => toast_event.show(&msg),
                AppEvent::Transfer(id, name, upload, state) => {
                    transfers_event.update(id, &name, upload, state);
                    let n = *transfers_event.active.borrow();
                    nav_transfers_ev.set_tooltip_text(if n > 0 { Some("Süren aktarımlar var") } else { None });
                }
                AppEvent::Error(err) => {
                    let err_copy = err.clone();
                    let t_copy = toast_event.clone();
                    widgets::error(&win_event, &err, move || {
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

    // Disconnect
    let conn_disc = Arc::clone(&active_connection);
    let (win_disc, log_disc) = (window.clone(), Rc::clone(&log_rev));
    let (conn_title_d, conn_dot_d, nav_hosts_d, nav_files_d) = (conn_title.clone(), conn_dot.clone(), nav_hosts.clone(), nav_files.clone());
    let hosts_reload = Rc::clone(&hosts.reload);
    btn_disconnect.connect_clicked(move |_| {
        *conn_disc.lock().unwrap() = None;
        conn_title_d.set_label("Bağlı değil");
        conn_dot_d.remove_css_class("online");
        win_disc.set_title(Some("Shift"));
        nav_hosts_d.set_active(true);
        nav_files_d.set_sensitive(false);
        nav_files_d.set_tooltip_text(Some("Önce bir sunucuya bağlanın"));
        hosts_reload();
        log_disc.append_log("status", "Bağlantı sonlandırıldı.");
    });

    // ------------------------------------------------------------- actions
    let act_site_mgr = gio::SimpleAction::new("site-manager", None);
    let (nh, nav_h) = (Rc::clone(&hosts.new_host), nav_hosts.clone());
    act_site_mgr.connect_activate(move |_, _| {
        nav_h.set_active(true);
        nh();
    });
    app.add_action(&act_site_mgr);
    app.set_accels_for_action("app.site-manager", &["<Control>n"]);

    let act_log = gio::SimpleAction::new("toggle-log", None);
    let nav_l = nav_log.clone();
    act_log.connect_activate(move |_, _| nav_l.set_active(true));
    app.add_action(&act_log);

    let initial_mode = THEME.with(|t| t.borrow().as_ref().map(|t| t.mode()).unwrap_or_else(|| "system".into()));
    let act_scheme = gio::SimpleAction::new_stateful("color-scheme", Some(glib::VariantTy::STRING), &initial_mode.to_variant());
    act_scheme.connect_activate(move |act, param| {
        let mode = param.and_then(|p| p.str()).unwrap_or("system").to_string();
        THEME.with(|t| {
            if let Some(t) = t.borrow().as_ref() {
                t.set_mode(&mode);
            }
        });
        act.set_state(&mode.to_variant());
    });
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
            .comments("FTP, FTPS ve SFTP dosya aktarım istemcisi.")
            .website("https://ensignsoft.com")
            .css_classes(["shift-dialog"])
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
