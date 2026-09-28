use crate::config::{Config, PatternEntry};
use crate::watcher::{self, LogMsg, WatcherHandle};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub struct SpineApp {
    config: Arc<Mutex<Config>>,
    watcher_handle: Option<WatcherHandle>,
    log_tx: Sender<LogMsg>,
    log_rx: Receiver<LogMsg>,
    log_lines: Vec<String>,

    new_pattern_name: String,
    new_pattern_regex: String,
    new_extension: String,
}

impl SpineApp {
    pub fn new() -> Self {
        let config = Arc::new(Mutex::new(Config::load()));
        let (log_tx, log_rx) = channel();
        SpineApp {
            config,
            watcher_handle: None,
            log_tx,
            log_rx,
            log_lines: Vec::new(),
            new_pattern_name: String::new(),
            new_pattern_regex: String::new(),
            new_extension: String::new(),
        }
    }

    fn is_watching(&self) -> bool {
        self.watcher_handle.is_some()
    }

    fn start_watching(&mut self) {
        self.watcher_handle = None; // stop any previous watcher first
        let cfg_snapshot = self.config.lock().unwrap().clone();
        watcher::initial_scan(&cfg_snapshot, &self.log_tx);
        match watcher::start(self.config.clone(), self.log_tx.clone()) {
            Ok(handle) => self.watcher_handle = Some(handle),
            Err(e) => {
                self.log_lines.push(format!("Erreur au demarrage: {}", e));
            }
        }
    }

    fn stop_watching(&mut self) {
        self.watcher_handle = None;
        self.log_lines.push("Surveillance arretee.".to_string());
    }

    fn drain_logs(&mut self) {
        while let Ok(msg) = self.log_rx.try_recv() {
            let line = match msg {
                LogMsg::Tagged { path, comment } => {
                    format!("[TAG] {} -> \"{}\"", path.display(), comment)
                }
                LogMsg::Info(s) => format!("[INFO] {}", s),
                LogMsg::Error(s) => format!("[ERREUR] {}", s),
            };
            self.log_lines.push(line);
            if self.log_lines.len() > 500 {
                self.log_lines.remove(0);
            }
        }
    }
}

impl eframe::App for SpineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_logs();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Spine");
            ui.label("Detecte des patterns (episodes, tomes...) dans les noms de fichiers et les ecrit dans le commentaire Finder, sans renommer les fichiers.");
            ui.separator();

            // --- Watched folder ---
            ui.horizontal(|ui| {
                ui.label("Dossier surveille:");
                let mut folder_str = self.config.lock().unwrap().watched_folder.display().to_string();
                let response = ui.text_edit_singleline(&mut folder_str);
                if response.changed() {
                    self.config.lock().unwrap().watched_folder = folder_str.into();
                }
                if ui.button("Parcourir...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.config.lock().unwrap().watched_folder = path;
                    }
                }
            });

            ui.horizontal(|ui| {
                if self.is_watching() {
                    ui.colored_label(egui::Color32::from_rgb(60, 170, 80), "Surveillance active");
                    if ui.button("Arreter").clicked() {
                        self.stop_watching();
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "Surveillance arretee");
                    if ui.button("Demarrer").clicked() {
                        self.start_watching();
                    }
                }
            });

            let mut overwrite = self.config.lock().unwrap().overwrite;
            if ui.checkbox(&mut overwrite, "Ecraser le commentaire existant (sinon, ajouter le pattern devant)").changed() {
                self.config.lock().unwrap().overwrite = overwrite;
                self.config.lock().unwrap().save();
            }

            ui.separator();

            // --- Patterns ---
            ui.label("Patterns detectes (regex):");
            let mut to_remove: Option<usize> = None;
            {
                let mut cfg = self.config.lock().unwrap();
                let mut changed = false;
                egui::Grid::new("patterns_grid").num_columns(4).striped(true).show(ui, |ui| {
                    for (i, pattern) in cfg.patterns.iter_mut().enumerate() {
                        if ui.checkbox(&mut pattern.enabled, "").changed() {
                            changed = true;
                        }
                        if ui
                            .add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut pattern.name))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .add_sized([320.0, 20.0], egui::TextEdit::singleline(&mut pattern.regex))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui.button("Supprimer").clicked() {
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
                ui.text_edit_singleline(&mut self.new_pattern_name).on_hover_text("Nom");
                ui.text_edit_singleline(&mut self.new_pattern_regex).on_hover_text("Regex");
                if ui.button("Ajouter un pattern").clicked() && !self.new_pattern_regex.trim().is_empty() {
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
            ui.label("Extensions de fichiers surveillees:");
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
                if ui.button("Ajouter une extension").clicked() && !self.new_extension.trim().is_empty() {
                    let ext = self.new_extension.trim().trim_start_matches('.').to_string();
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
            ui.label("Activite:");
            egui::ScrollArea::vertical().max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
                for line in &self.log_lines {
                    ui.monospace(line);
                }
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.lock().unwrap().save();
    }
}
