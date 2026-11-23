#![allow(deprecated)]
#![allow(dead_code)]
use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;

use crate::models::FileItem;

pub struct ExplorerView {
    pub container: gtk4::Box,
    pub list_store: gtk4::ListStore,
    pub path_box: gtk4::Box,
    pub search_entry: gtk4::SearchEntry,
    pub btn_back: gtk4::Button,
    pub btn_forward: gtk4::Button,
    pub btn_up: gtk4::Button,
    pub current_path: Rc<RefCell<String>>,
    pub history_back: Rc<RefCell<Vec<String>>>,
    pub history_forward: Rc<RefCell<Vec<String>>>,
    pub raw_items: Rc<RefCell<Vec<FileItem>>>,
    pub show_hidden: Rc<RefCell<bool>>,
}

fn get_file_icon(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".zip") || lower.ends_with(".tar") || lower.ends_with(".gz")
        || lower.ends_with(".xz") || lower.ends_with(".bz2") || lower.ends_with(".7z") {
        crate::ui::icons::pick(crate::ui::icons::ARCHIVE)
    } else if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
        || lower.ends_with(".svg") || lower.ends_with(".webp") || lower.ends_with(".gif") {
        crate::ui::icons::pick(crate::ui::icons::IMAGE)
    } else {
        crate::ui::icons::pick(crate::ui::icons::FILE)
    }
}

impl ExplorerView {
    pub fn new<FNav, FUpload, FDownload, FNewFolder, FDelete, FRename>(
        on_navigate: FNav,
        on_upload: FUpload,
        on_download: FDownload,
        on_new_folder: FNewFolder,
        on_delete: FDelete,
        on_rename: FRename,
    ) -> Self
    where
        FNav: Fn(String) + 'static + Clone,
        FUpload: Fn(String) + 'static + Clone,
        FDownload: Fn(FileItem) + 'static + Clone,
        FNewFolder: Fn(String) + 'static + Clone,
        FDelete: Fn(String, bool) + 'static + Clone,
        FRename: Fn(String, String) + 'static + Clone,
    {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        let current_path = Rc::new(RefCell::new("/".to_string()));
        let history_back: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let history_forward: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let raw_items = Rc::new(RefCell::new(Vec::new()));
        let show_hidden = Rc::new(RefCell::new(false));

        // 1. Navigation Action Bar
        let action_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let btn_back = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::BACK));
        btn_back.add_css_class("flat");
        btn_back.add_css_class("circular");
        btn_back.add_css_class("nav-icon-btn");
        btn_back.set_tooltip_text(Some("Geri Git"));
        btn_back.set_sensitive(false);
        action_bar.append(&btn_back);

        let btn_forward = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::FORWARD));
        btn_forward.add_css_class("flat");
        btn_forward.add_css_class("circular");
        btn_forward.add_css_class("nav-icon-btn");
        btn_forward.set_tooltip_text(Some("İleri Git"));
        btn_forward.set_sensitive(false);
        action_bar.append(&btn_forward);

        let btn_up = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::UP));
        btn_up.add_css_class("flat");
        btn_up.add_css_class("circular");
        btn_up.add_css_class("nav-icon-btn");
        btn_up.set_tooltip_text(Some("Üst Dizine Git"));
        btn_up.set_sensitive(false);
        action_bar.append(&btn_up);

        let path_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();
        action_bar.append(&path_box);

        // Upload Button
        let btn_upload = gtk4::Button::builder()
            .css_classes(["suggested-action", "pill"])
            .build();
        let up_content = libadwaita::ButtonContent::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::UPLOAD))
            .label("Dosya Yükle")
            .build();
        btn_upload.set_child(Some(&up_content));
        let on_up_clone = on_upload.clone();
        let cur_p_up = Rc::clone(&current_path);
        btn_upload.connect_clicked(move |_| on_up_clone(cur_p_up.borrow().clone()));
        action_bar.append(&btn_upload);

        // New Folder Button
        let btn_new_folder = gtk4::Button::builder()
            .css_classes(["flat", "pill"])
            .build();
        let folder_content = libadwaita::ButtonContent::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::FOLDER_NEW))
            .label("Yeni Klasör")
            .build();
        btn_new_folder.set_child(Some(&folder_content));
        let on_nf_clone = on_new_folder.clone();
        let cur_p_clone = Rc::clone(&current_path);
        btn_new_folder.connect_clicked(move |_| {
            on_nf_clone(cur_p_clone.borrow().clone());
        });
        action_bar.append(&btn_new_folder);

        // Refresh Button
        let btn_refresh = gtk4::Button::from_icon_name(crate::ui::icons::pick(crate::ui::icons::REFRESH));
        btn_refresh.add_css_class("flat");
        btn_refresh.add_css_class("circular");
        btn_refresh.set_tooltip_text(Some("Dizini Yenile"));
        let on_nav_ref = on_navigate.clone();
        let cur_p_ref = Rc::clone(&current_path);
        btn_refresh.connect_clicked(move |_| {
            on_nav_ref(cur_p_ref.borrow().clone());
        });
        action_bar.append(&btn_refresh);

        // Search Entry: 155px width
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text("Dosyalarda ara...")
            .width_request(155)
            .build();
        action_bar.append(&search_entry);

        // Hidden files toggle
        let btn_hidden = gtk4::ToggleButton::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::SHOW_HIDDEN))
            .tooltip_text("Gizli Dosyaları Göster/Gizle")
            .css_classes(["flat", "circular"])
            .build();
        action_bar.append(&btn_hidden);

        container.append(&action_bar);
        container.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // 2. Classic Flat Table View
        // Columns: 0: name, 1: size_str, 2: perms, 3: date, 4: icon, 5: is_dir, 6: path, 7: raw_size
        let list_store = gtk4::ListStore::new(&[
            glib::Type::STRING,
            glib::Type::STRING,
            glib::Type::STRING,
            glib::Type::STRING,
            glib::Type::STRING,
            glib::Type::BOOL,
            glib::Type::STRING,
            glib::Type::U64,
        ]);

        let tree_view = gtk4::TreeView::builder()
            .model(&list_store)
            .enable_tree_lines(false)
            .show_expanders(false)
            .headers_visible(true)
            .build();

        // Col 1: Name with Lucide Icon
        let col_name = gtk4::TreeViewColumn::builder()
            .title("Dosya Adı")
            .expand(true)
            .resizable(true)
            .build();
        let pix_cell = gtk4::CellRendererPixbuf::new();
        col_name.pack_start(&pix_cell, false);
        col_name.add_attribute(&pix_cell, "icon-name", 4);

        let text_cell = gtk4::CellRendererText::new();
        col_name.pack_start(&text_cell, true);
        col_name.add_attribute(&text_cell, "text", 0);
        tree_view.append_column(&col_name);

        // Col 2: Size
        let col_size = gtk4::TreeViewColumn::builder()
            .title("Boyut")
            .fixed_width(110)
            .resizable(true)
            .build();
        let size_cell = gtk4::CellRendererText::builder().xalign(1.0).build();
        col_size.pack_start(&size_cell, true);
        col_size.add_attribute(&size_cell, "text", 1);
        tree_view.append_column(&col_size);

        // Col 3: Permissions
        let col_perms = gtk4::TreeViewColumn::builder()
            .title("İzinler")
            .fixed_width(110)
            .resizable(true)
            .build();
        let perms_cell = gtk4::CellRendererText::builder().family("monospace").build();
        col_perms.pack_start(&perms_cell, true);
        col_perms.add_attribute(&perms_cell, "text", 2);
        tree_view.append_column(&col_perms);

        // Col 4: Date
        let col_date = gtk4::TreeViewColumn::builder()
            .title("Değiştirilme")
            .fixed_width(140)
            .resizable(true)
            .build();
        let date_cell = gtk4::CellRendererText::new();
        col_date.pack_start(&date_cell, true);
        col_date.add_attribute(&date_cell, "text", 3);
        tree_view.append_column(&col_date);

        // Navigation button logic
        let on_nav_back = on_navigate.clone();
        let h_back_b = Rc::clone(&history_back);
        let h_fwd_b = Rc::clone(&history_forward);
        let cur_p_b = Rc::clone(&current_path);
        btn_back.connect_clicked(move |_| {
            if let Some(prev) = h_back_b.borrow_mut().pop() {
                h_fwd_b.borrow_mut().push(cur_p_b.borrow().clone());
                *cur_p_b.borrow_mut() = prev.clone();
                on_nav_back(prev);
            }
        });

        let on_nav_fwd = on_navigate.clone();
        let h_back_f = Rc::clone(&history_back);
        let h_fwd_f = Rc::clone(&history_forward);
        let cur_p_f = Rc::clone(&current_path);
        btn_forward.connect_clicked(move |_| {
            if let Some(next) = h_fwd_f.borrow_mut().pop() {
                h_back_f.borrow_mut().push(cur_p_f.borrow().clone());
                *cur_p_f.borrow_mut() = next.clone();
                on_nav_fwd(next);
            }
        });

        let on_nav_up = on_navigate.clone();
        let h_back_u = Rc::clone(&history_back);
        let h_fwd_u = Rc::clone(&history_forward);
        let cur_p_u = Rc::clone(&current_path);
        btn_up.connect_clicked(move |_| {
            let cur = cur_p_u.borrow().clone();
            if cur != "/" && !cur.is_empty() {
                let parent = std::path::Path::new(&cur)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".into());
                h_back_u.borrow_mut().push(cur);
                h_fwd_u.borrow_mut().clear();
                *cur_p_u.borrow_mut() = parent.clone();
                on_nav_up(parent);
            }
        });

        // Double Click to Open / Download
        let ls_clone = list_store.clone();
        let on_nav_act = on_navigate.clone();
        let on_dl_act = on_download.clone();
        let cur_p_act = Rc::clone(&current_path);
        let h_back_act = Rc::clone(&history_back);
        let h_fwd_act = Rc::clone(&history_forward);

        tree_view.connect_row_activated(move |_, path, _| {
            if let Some(iter) = ls_clone.iter(path) {
                let name: String = ls_clone.get(&iter, 0);
                let is_dir: bool = ls_clone.get(&iter, 5);
                let full_path: String = ls_clone.get(&iter, 6);
                let raw_size: u64 = ls_clone.get(&iter, 7);

                if name == ".." {
                    let parent = std::path::Path::new(&*cur_p_act.borrow())
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/".into());
                    h_back_act.borrow_mut().push(cur_p_act.borrow().clone());
                    h_fwd_act.borrow_mut().clear();
                    *cur_p_act.borrow_mut() = parent.clone();
                    on_nav_act(parent);
                } else if is_dir {
                    h_back_act.borrow_mut().push(cur_p_act.borrow().clone());
                    h_fwd_act.borrow_mut().clear();
                    *cur_p_act.borrow_mut() = full_path.clone();
                    on_nav_act(full_path);
                } else {
                    on_dl_act(FileItem {
                        name,
                        is_dir: false,
                        size: raw_size,
                        modified_time: 0,
                        permissions: "".into(),
                        path: full_path,
                    });
                }
            }
        });

        // Right-Click Context Menu (Arrow-free, left-aligned with Lucide icons)
        let gesture_click = gtk4::GestureClick::builder()
            .button(3)
            .build();

        let ls_ctx = list_store.clone();
        let tv_ctx = tree_view.clone();
        let cur_p_ctx = Rc::clone(&current_path);
        let on_nav_ctx = on_navigate.clone();
        let on_up_ctx = on_upload.clone();
        let on_dl_ctx = on_download.clone();
        let on_nf_ctx = on_new_folder.clone();
        let on_del_ctx = on_delete.clone();
        let on_ren_ctx = on_rename.clone();

        gesture_click.connect_pressed(move |_, _n_press, x, y| {
            let popover = gtk4::Popover::builder()
                .has_arrow(false)
                .autohide(true)
                .build();
            popover.set_parent(&tv_ctx);
            popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

            let menu_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(3)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(6)
                .margin_end(6)
                .width_request(200)
                .build();

            let row_info = if let Some((Some(path), _, _, _)) = tv_ctx.path_at_pos(x as i32, y as i32) {
                tv_ctx.selection().select_path(&path);
                if let Some(iter) = ls_ctx.iter(&path) {
                    let name: String = ls_ctx.get(&iter, 0);
                    let is_dir: bool = ls_ctx.get(&iter, 5);
                    let full_path: String = ls_ctx.get(&iter, 6);
                    let raw_size: u64 = ls_ctx.get(&iter, 7);
                    if name != ".." {
                        Some((name, is_dir, full_path, raw_size))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let add_menu_item = |box_ref: &gtk4::Box, pop_ref: &gtk4::Popover, label: &str, icon: &str, destructive: bool, action: Box<dyn Fn()>| {
                let btn = gtk4::Button::builder()
                    .css_classes(if destructive { vec!["flat", "error"] } else { vec!["flat"] })
                    .halign(gtk4::Align::Fill)
                    .build();
                let content = libadwaita::ButtonContent::builder()
                    .icon_name(icon)
                    .label(label)
                    .halign(gtk4::Align::Start)
                    .margin_start(4)
                    .margin_end(8)
                    .build();
                btn.set_child(Some(&content));

                let pop_c = pop_ref.clone();
                btn.connect_clicked(move |_| {
                    pop_c.popdown();
                    action();
                });
                box_ref.append(&btn);
            };

            if let Some((name, is_dir, full_path, raw_size)) = row_info {
                if is_dir {
                    // Open Folder
                    let on_nav_c = on_nav_ctx.clone();
                    let fp_c = full_path.clone();
                    add_menu_item(&menu_box, &popover, "Klasörü Aç", crate::ui::icons::pick(crate::ui::icons::FOLDER_OPEN), false, Box::new(move || {
                        on_nav_c(fp_c.clone());
                    }));
                } else {
                    // Download File
                    let on_dl_c = on_dl_ctx.clone();
                    let file_it = FileItem {
                        name: name.clone(),
                        is_dir: false,
                        size: raw_size,
                        modified_time: 0,
                        permissions: "".into(),
                        path: full_path.clone(),
                    };
                    add_menu_item(&menu_box, &popover, "Bilgisayara İndir", crate::ui::icons::pick(crate::ui::icons::DOWNLOAD), false, Box::new(move || {
                        on_dl_c(file_it.clone());
                    }));
                }

                // Rename
                let on_ren_c = on_ren_ctx.clone();
                let fp_ren = full_path.clone();
                add_menu_item(&menu_box, &popover, "Yeniden Adlandır", crate::ui::icons::pick(crate::ui::icons::RENAME), false, Box::new(move || {
                    on_ren_c(fp_ren.clone(), String::new());
                }));

                // Refresh
                let on_nav_ref2 = on_nav_ctx.clone();
                let cur_p2 = cur_p_ctx.borrow().clone();
                add_menu_item(&menu_box, &popover, "Dizini Yenile", crate::ui::icons::pick(crate::ui::icons::REFRESH), false, Box::new(move || {
                    on_nav_ref2(cur_p2.clone());
                }));

                // New Folder
                let on_nf_c2 = on_nf_ctx.clone();
                let cur_p3 = cur_p_ctx.borrow().clone();
                add_menu_item(&menu_box, &popover, "Yeni Klasör Oluştur", crate::ui::icons::pick(crate::ui::icons::FOLDER_NEW), false, Box::new(move || {
                    on_nf_c2(cur_p3.clone());
                }));

                menu_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

                // Delete
                let on_del_c = on_del_ctx.clone();
                let fp_del = full_path.clone();
                add_menu_item(&menu_box, &popover, "Sil", crate::ui::icons::pick(crate::ui::icons::DELETE), true, Box::new(move || {
                    on_del_c(fp_del.clone(), is_dir);
                }));
            } else {
                // Background empty space click
                let on_up_c3 = on_up_ctx.clone();
                let cur_p_up_bg = cur_p_ctx.borrow().clone();
                add_menu_item(&menu_box, &popover, "Dosya Yükle", crate::ui::icons::pick(crate::ui::icons::UPLOAD), false, Box::new(move || {
                    on_up_c3(cur_p_up_bg.clone());
                }));

                let on_nf_c3 = on_nf_ctx.clone();
                let cur_p4 = cur_p_ctx.borrow().clone();
                add_menu_item(&menu_box, &popover, "Yeni Klasör Oluştur", crate::ui::icons::pick(crate::ui::icons::FOLDER_NEW), false, Box::new(move || {
                    on_nf_c3(cur_p4.clone());
                }));

                let on_nav_ref3 = on_nav_ctx.clone();
                let cur_p5 = cur_p_ctx.borrow().clone();
                add_menu_item(&menu_box, &popover, "Dizini Yenile", crate::ui::icons::pick(crate::ui::icons::REFRESH), false, Box::new(move || {
                    on_nav_ref3(cur_p5.clone());
                }));
            }

            popover.set_child(Some(&menu_box));
            popover.popup();
        });
        tree_view.add_controller(gesture_click);

        let scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&tree_view)
            .build();
        container.append(&scrolled);

        let exp = Self {
            container,
            list_store,
            path_box,
            search_entry: search_entry.clone(),
            btn_back,
            btn_forward,
            btn_up,
            current_path,
            history_back,
            history_forward,
            raw_items,
            show_hidden: Rc::clone(&show_hidden),
        };

        // Wire search filter
        let exp_rc = Rc::new(exp);
        let exp_search = Rc::clone(&exp_rc);
        search_entry.connect_search_changed(move |_| {
            exp_search.populate();
        });

        // Wire hidden files button
        let exp_hid = Rc::clone(&exp_rc);
        let btn_hid_ref = btn_hidden.clone();
        btn_hidden.connect_toggled(move |btn| {
            let active = btn.is_active();
            *exp_hid.show_hidden.borrow_mut() = active;
            if active {
                btn_hid_ref.set_icon_name(crate::ui::icons::pick(crate::ui::icons::HIDE_HIDDEN));
            } else {
                btn_hid_ref.set_icon_name(crate::ui::icons::pick(crate::ui::icons::SHOW_HIDDEN));
            }
            exp_hid.populate();
        });

        match Rc::try_unwrap(exp_rc) {
            Ok(unwrapped) => unwrapped,
            Err(rc) => (*rc).clone_struct(),
        }
    }

    fn clone_struct(&self) -> Self {
        Self {
            container: self.container.clone(),
            list_store: self.list_store.clone(),
            path_box: self.path_box.clone(),
            search_entry: self.search_entry.clone(),
            btn_back: self.btn_back.clone(),
            btn_forward: self.btn_forward.clone(),
            btn_up: self.btn_up.clone(),
            current_path: Rc::clone(&self.current_path),
            history_back: Rc::clone(&self.history_back),
            history_forward: Rc::clone(&self.history_forward),
            raw_items: Rc::clone(&self.raw_items),
            show_hidden: Rc::clone(&self.show_hidden),
        }
    }

    pub fn set_items(&self, path: &str, items: Vec<FileItem>) {
        *self.current_path.borrow_mut() = path.to_string();
        *self.raw_items.borrow_mut() = items;
        self.btn_back.set_sensitive(!self.history_back.borrow().is_empty());
        self.btn_forward.set_sensitive(!self.history_forward.borrow().is_empty());
        self.btn_up.set_sensitive(path != "/" && !path.is_empty());
        self.update_breadcrumbs(path);
        self.populate();
    }

    pub fn populate(&self) {
        self.list_store.clear();
        let cur = self.current_path.borrow().clone();

        if cur != "/" && !cur.is_empty() {
            let parent = std::path::Path::new(&cur)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".into());
            self.list_store.insert_with_values(
                None,
                &[
                    (0, &".."),
                    (1, &"Üst Dizin"),
                    (2, &""),
                    (3, &""),
                    (4, &crate::ui::icons::pick(crate::ui::icons::FOLDER)),
                    (5, &true),
                    (6, &parent),
                    (7, &0u64),
                ],
            );
        }

        let query = self.search_entry.text().to_lowercase();
        let show_hid = *self.show_hidden.borrow();

        for it in self.raw_items.borrow().iter() {
            if !show_hid && it.name.starts_with('.') {
                continue;
            }
            if !query.is_empty() && !it.name.to_lowercase().contains(&query) {
                continue;
            }

            let size_str = if it.is_dir {
                "Dizin".to_string()
            } else if it.size < 1024 {
                format!("{} B", it.size)
            } else if it.size < 1024 * 1024 {
                format!("{:.1} KB", it.size as f64 / 1024.0)
            } else {
                format!("{:.1} MB", it.size as f64 / (1024.0 * 1024.0))
            };

            let icon = if it.is_dir {
                crate::ui::icons::pick(crate::ui::icons::FOLDER)
            } else {
                get_file_icon(&it.name)
            };

            let date_str = chrono::DateTime::from_timestamp(it.modified_time, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            self.list_store.insert_with_values(
                None,
                &[
                    (0, &it.name),
                    (1, &size_str),
                    (2, &it.permissions),
                    (3, &date_str),
                    (4, &icon),
                    (5, &it.is_dir),
                    (6, &it.path),
                    (7, &it.size),
                ],
            );
        }
    }

    pub fn update_breadcrumbs(&self, path: &str) {
        while let Some(child) = self.path_box.first_child() {
            self.path_box.remove(&child);
        }

        let btn_root = gtk4::Button::builder().css_classes(["flat"]).build();
        let root_content = libadwaita::ButtonContent::builder()
            .icon_name(crate::ui::icons::pick(crate::ui::icons::SERVER))
            .label("Sunucu Kökü")
            .build();
        btn_root.set_child(Some(&root_content));
        self.path_box.append(&btn_root);

        let clean = path.trim_matches('/');
        if clean.is_empty() {
            return;
        }

        let parts: Vec<&str> = clean.split('/').collect();
        for p in parts {
            let sep = gtk4::Image::from_icon_name(crate::ui::icons::pick(crate::ui::icons::SEPARATOR));
            sep.set_pixel_size(12);
            sep.add_css_class("dim-label");
            self.path_box.append(&sep);

            let btn_seg = gtk4::Button::builder().label(p).css_classes(["flat"]).build();
            self.path_box.append(&btn_seg);
        }
    }
}
