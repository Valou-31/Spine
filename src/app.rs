use crate::config::{Config, PatternEntry};
use crate::i18n::Language;
use crate::watcher::{self, LogMsg, Waker, WatcherHandle};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Shows/hides the Dock icon (and Cmd+Tab entry) to match the window's
/// visibility, so hiding the window to the menu bar really leaves only the
/// menu bar icon behind instead of a Dock icon with no visible window.
fn set_dock_visible(visible: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    app.setActivationPolicy(policy);
}

fn load_tray_icon() -> Option<tray_icon::Icon> {
    // Rasterized from assets/logo.svg at build time by build.rs.
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/tray_icon.png"));
    let img = image::load_from_memory(bytes)
        .ok()?
        .resize(44, 44, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    let (width, height) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), width, height).ok()
}

fn build_tray_icon(lang: Language) -> Option<(TrayIcon, MenuItem, MenuItem)> {
    let menu = Menu::new();
    let toggle_item = MenuItem::with_id("toggle", lang.tray_toggle(), true, None);
    let quit_item = MenuItem::with_id("quit", lang.tray_quit(), true, None);
    menu.append(&toggle_item).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit_item).ok()?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_icon(load_tray_icon()?)
        .with_icon_as_template(true)
        .with_tooltip("Spine")
        .build()
        .ok()?;
    Some((tray_icon, toggle_item, quit_item))
}

pub struct SpineApp {
    config: Arc<Mutex<Config>>,
    watcher_handle: Option<WatcherHandle>,
    log_tx: Sender<LogMsg>,
    log_rx: Receiver<LogMsg>,
    log_lines: VecDeque<String>,
    window_visible: bool,
    waker: Waker,
    _tray_icon: Option<TrayIcon>,
    tray_toggle_item: Option<MenuItem>,
    tray_quit_item: Option<MenuItem>,
    tray_lang: Language,
    tray_event_rx: Receiver<TrayIconEvent>,
    menu_event_rx: Receiver<MenuEvent>,

    new_pattern_name: String,
    new_pattern_regex: String,
    new_extension: String,
}

impl SpineApp {
    /// The app never requests periodic repaints: everything that can happen
    /// in the background (watcher activity, tray/menu clicks) wakes the UI
    /// explicitly via this `ctx.request_repaint()`, so an idle app truly does
    /// nothing between user interactions instead of redrawing on a timer.
    pub fn new(ctx: &egui::Context) -> Self {
        let config = Arc::new(Mutex::new(Config::load()));
        let lang = config.lock().unwrap().language;
        let (log_tx, log_rx) = channel();
        let waker: Waker = {
            let ctx = ctx.clone();
            Arc::new(move || ctx.request_repaint())
        };

        let (tray_tx, tray_event_rx) = channel();
        let tray_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            tray_tx.send(event).ok();
            tray_ctx.request_repaint();
        }));

        let (menu_tx, menu_event_rx) = channel();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            menu_tx.send(event).ok();
            menu_ctx.request_repaint();
        }));

        let (tray_icon, toggle_item, quit_item) = match build_tray_icon(lang) {
            Some((icon, toggle, quit)) => (Some(icon), Some(toggle), Some(quit)),
            None => (None, None, None),
        };

        SpineApp {
            config,
            watcher_handle: None,
            log_tx,
            log_rx,
            log_lines: VecDeque::new(),
            window_visible: true,
            waker,
            _tray_icon: tray_icon,
            tray_toggle_item: toggle_item,
            tray_quit_item: quit_item,
            tray_lang: lang,
            tray_event_rx,
            menu_event_rx,
            new_pattern_name: String::new(),
            new_pattern_regex: String::new(),
            new_extension: String::new(),
        }
    }

    fn toggle_window(&mut self, ctx: &egui::Context) {
        self.window_visible = !self.window_visible;
        set_dock_visible(self.window_visible);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.window_visible));
        if self.window_visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        if let Ok(TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }) = self.tray_event_rx.try_recv()
        {
            self.toggle_window(ctx);
        }

        if let Ok(event) = self.menu_event_rx.try_recv() {
            match event.id.as_ref() {
                "toggle" => self.toggle_window(ctx),
                "quit" => {
                    self.config.lock().unwrap().save();
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    }

    fn is_watching(&self) -> bool {
        self.watcher_handle.is_some()
    }

    fn start_watching(&mut self) {
        self.watcher_handle = None; // stop any previous watcher first
        self.spawn_scan();
        let lang = self.config.lock().unwrap().language;
        match watcher::start(self.config.clone(), self.log_tx.clone(), self.waker.clone()) {
            Ok(handle) => self.watcher_handle = Some(handle),
            Err(e) => {
                self.log_lines.push_back(lang.start_error(&e.to_string()));
            }
        }
    }

    fn stop_watching(&mut self) {
        self.watcher_handle = None;
        let lang = self.config.lock().unwrap().language;
        self.log_lines
            .push_back(lang.watching_stopped_log().to_string());
    }

    /// Keeps the tray menu item labels in sync with the current language,
    /// since they're created once at startup but the language can change
    /// live while the app is running.
    fn sync_tray_language(&mut self) {
        let lang = self.config.lock().unwrap().language;
        if lang == self.tray_lang {
            return;
        }
        if let Some(item) = &self.tray_toggle_item {
            item.set_text(lang.tray_toggle());
        }
        if let Some(item) = &self.tray_quit_item {
            item.set_text(lang.tray_quit());
        }
        self.tray_lang = lang;
    }

    /// Runs the folder scan on a background thread so the UI never freezes,
    /// even on a large library. Can also be triggered manually as a fallback
    /// rescan, in case a filesystem event was ever missed.
    fn spawn_scan(&self) {
        let cfg_snapshot = self.config.lock().unwrap().clone();
        let log_tx = self.log_tx.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            watcher::initial_scan(&cfg_snapshot, &log_tx, &waker);
        });
    }

    fn drain_logs(&mut self) {
        let lang = self.config.lock().unwrap().language;
        while let Ok(msg) = self.log_rx.try_recv() {
            let line = match msg {
                LogMsg::Tagged { path, comment } => {
                    lang.log_tag(&path.display().to_string(), &comment)
                }
                LogMsg::Info(s) => lang.log_info(&s),
                LogMsg::Error(s) => lang.log_error(&s),
            };
            self.log_lines.push_back(line);
            if self.log_lines.len() > 500 {
                self.log_lines.pop_front();
            }
        }
    }
}

impl eframe::App for SpineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_logs();
        self.handle_tray_events(ctx);
        self.sync_tray_language();

        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            set_dock_visible(false);
            self.window_visible = false;
        }

        let lang = self.config.lock().unwrap().language;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Spine");
                ui.add_space(8.0);
                if ui
                    .selectable_label(lang == Language::Fr, Language::Fr.flag())
                    .on_hover_text(Language::Fr.native_name())
                    .clicked()
                {
                    let mut cfg = self.config.lock().unwrap();
                    cfg.language = Language::Fr;
                    cfg.save();
                }
                if ui
                    .selectable_label(lang == Language::En, Language::En.flag())
                    .on_hover_text(Language::En.native_name())
                    .clicked()
                {
                    let mut cfg = self.config.lock().unwrap();
                    cfg.language = Language::En;
                    cfg.save();
                }
            });
            ui.label(lang.description());
            ui.small(lang.hint_hide());
            ui.separator();

            // --- Watched folder ---
            ui.horizontal(|ui| {
                ui.label(lang.watched_folder_label());
                let mut folder_str = self
                    .config
                    .lock()
                    .unwrap()
                    .watched_folder
                    .display()
                    .to_string();
                let response = ui.text_edit_singleline(&mut folder_str);
                if response.changed() {
                    self.config.lock().unwrap().watched_folder = folder_str.into();
                }
                if ui.button(lang.browse()).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.config.lock().unwrap().watched_folder = path;
                    }
                }
            });

            ui.horizontal(|ui| {
                if self.is_watching() {
                    ui.colored_label(egui::Color32::from_rgb(60, 170, 80), lang.watching_active());
                    if ui.button(lang.stop()).clicked() {
                        self.stop_watching();
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, lang.watching_stopped());
                    if ui.button(lang.start()).clicked() {
                        self.start_watching();
                    }
                }
                if ui
                    .button(lang.rescan())
                    .on_hover_text(lang.rescan_hover())
                    .clicked()
                {
                    self.spawn_scan();
                }
            });

            let mut overwrite = self.config.lock().unwrap().overwrite;
            if ui
                .checkbox(&mut overwrite, lang.overwrite_checkbox())
                .changed()
            {
                self.config.lock().unwrap().overwrite = overwrite;
                self.config.lock().unwrap().save();
            }

            ui.separator();

            // --- Patterns ---
            ui.label(lang.patterns_label());
            let mut to_remove: Option<usize> = None;
            {
                let mut cfg = self.config.lock().unwrap();
                let mut changed = false;
                egui::Grid::new("patterns_grid")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        for (i, pattern) in cfg.patterns.iter_mut().enumerate() {
                            if ui.checkbox(&mut pattern.enabled, "").changed() {
                                changed = true;
                            }
                            if ui
                                .add_sized(
                                    [150.0, 20.0],
                                    egui::TextEdit::singleline(&mut pattern.name),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            if ui
                                .add_sized(
                                    [320.0, 20.0],
                                    egui::TextEdit::singleline(&mut pattern.regex),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            if ui.button(lang.delete()).clicked() {
                                to_remove = Some(i);
                            }
                            ui.end_row();
                        }
                    });
                if let Some(i) = to_remove {
                    cfg.patterns.remove(i);
                    changed = true;
                }
                if changed {
                    cfg.save();
                }
            }

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_pattern_name)
                    .on_hover_text(lang.pattern_name_hover());
                ui.text_edit_singleline(&mut self.new_pattern_regex)
                    .on_hover_text(lang.pattern_regex_hover());
                if ui.button(lang.add_pattern()).clicked()
                    && !self.new_pattern_regex.trim().is_empty()
                {
                    let name = if self.new_pattern_name.trim().is_empty() {
                        self.new_pattern_regex.clone()
                    } else {
                        self.new_pattern_name.clone()
                    };
                    let mut cfg = self.config.lock().unwrap();
                    cfg.patterns.push(PatternEntry {
                        name,
                        regex: self.new_pattern_regex.clone(),
                        enabled: true,
                    });
                    cfg.save();
                    self.new_pattern_name.clear();
                    self.new_pattern_regex.clear();
                }
            });

            ui.separator();

            // --- Extensions ---
            ui.label(lang.extensions_label());
            let mut ext_to_remove: Option<usize> = None;
            {
                let mut cfg = self.config.lock().unwrap();
                ui.horizontal_wrapped(|ui| {
                    for (i, ext) in cfg.extensions.iter().enumerate() {
                        ui.group(|ui| {
                            ui.label(ext);
                            if ui.small_button("x").clicked() {
                                ext_to_remove = Some(i);
                            }
                        });
                    }
                });
                if let Some(i) = ext_to_remove {
                    cfg.extensions.remove(i);
                    cfg.save();
                }
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_extension);
                if ui.button(lang.add_extension()).clicked()
                    && !self.new_extension.trim().is_empty()
                {
                    let ext = self
                        .new_extension
                        .trim()
                        .trim_start_matches('.')
                        .to_string();
                    let mut cfg = self.config.lock().unwrap();
                    if !cfg.extensions.iter().any(|e| e.eq_ignore_ascii_case(&ext)) {
                        cfg.extensions.push(ext);
                        cfg.save();
                    }
                    self.new_extension.clear();
                }
            });

            ui.separator();

            // --- Log ---
            ui.label(lang.activity_label());
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log_lines {
                        ui.monospace(line);
                    }
                });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.lock().unwrap().save();
    }
}
