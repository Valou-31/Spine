use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    En,
    Fr,
}

impl Language {
    pub fn flag(self) -> &'static str {
        match self {
            Language::En => "\u{1F1EC}\u{1F1E7}",
            Language::Fr => "\u{1F1EB}\u{1F1F7}",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Fr => "Français",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Language::En => "Detects patterns (episodes, volumes...) in filenames and writes them to the Finder comment, without renaming files.",
            Language::Fr => "Detecte des patterns (episodes, tomes...) dans les noms de fichiers et les ecrit dans le commentaire Finder, sans renommer les fichiers.",
        }
    }

    pub fn hint_hide(self) -> &'static str {
        match self {
            Language::En => "Tip: closing this window minimizes it to the menu bar (icon top right) without stopping the watcher.",
            Language::Fr => "Astuce : fermer cette fenetre la reduit dans la barre de menu (icone en haut a droite) sans arreter la surveillance.",
        }
    }

    pub fn watched_folder_label(self) -> &'static str {
        match self {
            Language::En => "Watched folder:",
            Language::Fr => "Dossier surveille:",
        }
    }

    pub fn browse(self) -> &'static str {
        match self {
            Language::En => "Browse...",
            Language::Fr => "Parcourir...",
        }
    }

    pub fn watching_active(self) -> &'static str {
        match self {
            Language::En => "Watching active",
            Language::Fr => "Surveillance active",
        }
    }

    pub fn stop(self) -> &'static str {
        match self {
            Language::En => "Stop",
            Language::Fr => "Arreter",
        }
    }

    pub fn watching_stopped(self) -> &'static str {
        match self {
            Language::En => "Watching stopped",
            Language::Fr => "Surveillance arretee",
        }
    }

    pub fn start(self) -> &'static str {
        match self {
            Language::En => "Start",
            Language::Fr => "Demarrer",
        }
    }

    pub fn rescan(self) -> &'static str {
        match self {
            Language::En => "Rescan",
            Language::Fr => "Rescanner",
        }
    }

    pub fn rescan_hover(self) -> &'static str {
        match self {
            Language::En => {
                "Runs a full scan of the folder again, in case a change was ever missed."
            }
            Language::Fr => {
                "Relance une analyse complete du dossier, au cas ou un changement aurait ete manque."
            }
        }
    }

    pub fn overwrite_checkbox(self) -> &'static str {
        match self {
            Language::En => "Overwrite existing comment (otherwise, prepend the pattern)",
            Language::Fr => "Ecraser le commentaire existant (sinon, ajouter le pattern devant)",
        }
    }

    pub fn patterns_label(self) -> &'static str {
        match self {
            Language::En => "Detected patterns (regex):",
            Language::Fr => "Patterns detectes (regex):",
        }
    }

    pub fn delete(self) -> &'static str {
        match self {
            Language::En => "Delete",
            Language::Fr => "Supprimer",
        }
    }

    pub fn add_pattern(self) -> &'static str {
        match self {
            Language::En => "Add a pattern",
            Language::Fr => "Ajouter un pattern",
        }
    }

    pub fn pattern_name_hover(self) -> &'static str {
        match self {
            Language::En => "Name",
            Language::Fr => "Nom",
        }
    }

    pub fn pattern_regex_hover(self) -> &'static str {
        "Regex"
    }

    pub fn extensions_label(self) -> &'static str {
        match self {
            Language::En => "Watched file extensions:",
            Language::Fr => "Extensions de fichiers surveillees:",
        }
    }

    pub fn add_extension(self) -> &'static str {
        match self {
            Language::En => "Add an extension",
            Language::Fr => "Ajouter une extension",
        }
    }

    pub fn activity_label(self) -> &'static str {
        match self {
            Language::En => "Activity:",
            Language::Fr => "Activite:",
        }
    }

    pub fn tray_toggle(self) -> &'static str {
        match self {
            Language::En => "Show / Hide Spine",
            Language::Fr => "Afficher / Masquer Spine",
        }
    }

    pub fn tray_quit(self) -> &'static str {
        match self {
            Language::En => "Quit Spine",
            Language::Fr => "Quitter Spine",
        }
    }

    pub fn start_error(self, err: &str) -> String {
        match self {
            Language::En => format!("Startup error: {err}"),
            Language::Fr => format!("Erreur au demarrage: {err}"),
        }
    }

    pub fn watching_stopped_log(self) -> &'static str {
        match self {
            Language::En => "Watching stopped.",
            Language::Fr => "Surveillance arretee.",
        }
    }

    pub fn initial_scan_start(self, folder: &str) -> String {
        match self {
            Language::En => format!("Scanning {folder}..."),
            Language::Fr => format!("Scan initial de {folder}..."),
        }
    }

    pub fn initial_scan_done(self, count: usize) -> String {
        match self {
            Language::En => format!("Initial scan done ({count} file(s) tagged)."),
            Language::Fr => format!("Scan initial termine ({count} fichier(s) tague(s))."),
        }
    }

    pub fn watching_active_log(self, folder: &str) -> String {
        match self {
            Language::En => format!("Watching {folder}"),
            Language::Fr => format!("Surveillance active sur {folder}"),
        }
    }

    pub fn log_tag(self, path: &str, comment: &str) -> String {
        format!("[TAG] {path} -> \"{comment}\"")
    }

    pub fn log_info(self, s: &str) -> String {
        format!("[INFO] {s}")
    }

    pub fn log_error(self, s: &str) -> String {
        match self {
            Language::En => format!("[ERROR] {s}"),
            Language::Fr => format!("[ERREUR] {s}"),
        }
    }
}
