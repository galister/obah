use crate::input_profiles::INPUT_PROFILES_BYTES;
use crate::types::manifest::SteamVrBindingFile;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSourceKind {
    Xrizer,
    OpenComposite,
    Vapor,
}

#[derive(Debug, Clone)]
pub struct DiscoveredBinding {
    pub controller_type: String,
    pub kind: BindingSourceKind,
}

impl BindingSourceKind {
    pub fn save_path(&self, game_folder: &Path, controller_type: &str) -> PathBuf {
        match self {
            BindingSourceKind::Xrizer => {
                let stripped = controller_type.replace('_', "");
                game_folder
                    .join("xrizer")
                    .join(format!("{}.json", stripped))
            }
            BindingSourceKind::OpenComposite => game_folder
                .join("OpenComposite")
                .join(format!("{}.json", controller_type)),
            BindingSourceKind::Vapor => game_folder.join("vapor_binding.json"),
        }
    }
}

fn strip_underscores(s: &str) -> String {
    s.replace('_', "")
}

fn scan_xrizer(game_folder: &Path, out: &mut Vec<DiscoveredBinding>) {
    let xrizer_dir = game_folder.join("xrizer");
    if !xrizer_dir.is_dir() {
        return;
    }

    let known_types: Vec<String> = INPUT_PROFILES_BYTES
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    let known_stripped: Vec<String> = known_types.iter().map(|t| strip_underscores(t)).collect();

    let entries = match std::fs::read_dir(&xrizer_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let filename = match path.file_stem().and_then(|s| s.to_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        let stripped = strip_underscores(&filename);
        if let Some(idx) = known_stripped.iter().position(|s| *s == stripped) {
            out.push(DiscoveredBinding {
                controller_type: known_types[idx].clone(),
                kind: BindingSourceKind::Xrizer,
            });
        }
    }
}

fn scan_open_composite(game_folder: &Path, out: &mut Vec<DiscoveredBinding>) {
    let oc_dir = game_folder.join("OpenComposite");
    if !oc_dir.is_dir() {
        return;
    }

    let known_types: Vec<&str> = INPUT_PROFILES_BYTES.iter().map(|(name, _)| *name).collect();

    let entries = match std::fs::read_dir(&oc_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let filename = match path.file_stem().and_then(|s| s.to_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        if known_types.contains(&filename.as_str()) {
            out.push(DiscoveredBinding {
                controller_type: filename,
                kind: BindingSourceKind::OpenComposite,
            });
        }
    }
}

fn scan_vapor(game_folder: &Path, out: &mut Vec<DiscoveredBinding>) {
    let vapor_path = game_folder.join("vapor_binding.json");
    if !vapor_path.is_file() {
        return;
    }

    let content = match std::fs::read_to_string(&vapor_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let binding: SteamVrBindingFile = match serde_json::from_str(&content) {
        Ok(b) => b,
        Err(_) => return,
    };

    out.push(DiscoveredBinding {
        controller_type: binding.controller_type,
        kind: BindingSourceKind::Vapor,
    });
}

pub fn scan_game_folder(game_folder: &Path) -> Vec<DiscoveredBinding> {
    let mut bindings = Vec::new();
    scan_xrizer(game_folder, &mut bindings);
    scan_open_composite(game_folder, &mut bindings);
    scan_vapor(game_folder, &mut bindings);
    bindings
}
