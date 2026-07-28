use keyvalues_parser::{Obj, Vdf};
use std::path::{Path, PathBuf};

const MANIFEST_NAMES: &[&str] = &[
    "actions.json",
    "action_manifest.json",
    "vr_actions.json",
    "steamvr_actions.json",
];

#[derive(Clone, Debug)]
pub struct AppManifest {
    pub name: String,
    pub actions_json_path: Option<PathBuf>,
    pub game_folder: PathBuf,
}

fn get_steam_root() -> anyhow::Result<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME")?);

    let steam_paths: [&str; 3] = [
        ".steam/steam",
        ".steam/debian-installation",
        ".var/app/com.valvesoftware.Steam/data/Steam",
    ];
    let Some(steam_path) = steam_paths
        .iter()
        .map(|p| home.join(p))
        .find(|p| p.exists())
    else {
        anyhow::bail!("Couldn't find Steam installation");
    };

    Ok(steam_path)
}

fn get_obj_first<'a>(obj: &'a Obj<'_>, key: &str) -> Option<&'a Obj<'a>> {
    obj.get(key)?.first()?.get_obj()
}

fn get_str_first<'a>(obj: &'a Obj<'_>, key: &str) -> Option<&'a str> {
    obj.get(key)?.first()?.get_str()
}

fn parse_libraryfolders(vdf_root: &Vdf) -> Option<Vec<AppEntry>> {
    let obj_libraryfolders = vdf_root.value.get_obj()?;

    let mut res = Vec::<AppEntry>::new();
    let mut num = 0;

    loop {
        let Some(library_folder) = get_obj_first(obj_libraryfolders, &format!("{}", num)) else {
            break;
        };

        let Some(apps) = get_obj_first(library_folder, "apps") else {
            num += 1;
            continue;
        };

        let Some(path) = get_str_first(library_folder, "path") else {
            num += 1;
            continue;
        };

        res.extend(
            apps.iter()
                .filter_map(|item| item.0.parse::<u64>().ok())
                .map(|app_id| AppEntry {
                    app_id,
                    root_path: String::from(path),
                }),
        );

        num += 1;
    }

    Some(res)
}

fn parse_app_name<'a>(app_state_obj: &'a Obj<'a>) -> Option<&'a str> {
    app_state_obj.get("name")?.first()?.get_str()
}

struct AppEntry {
    root_path: String,
    app_id: u64,
}

pub fn find_actions_json(game_path: &Path) -> Option<PathBuf> {
    if !game_path.is_dir() {
        return None;
    }

    fn recurse(current: &Path, depth: u32) -> Option<PathBuf> {
        if depth > 8 {
            return None;
        }

        let Ok(entries) = std::fs::read_dir(current) else {
            return None;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path
                .file_name()
                .map_or(false, |n| MANIFEST_NAMES.iter().any(|m| n == *m))
            {
                return Some(path);
            }

            if path.is_dir() {
                if let Some(found) = recurse(&path, depth + 1) {
                    return Some(found);
                }
            }
        }

        None
    }

    recurse(game_path, 0)
}

pub fn list_steamvr_games() -> anyhow::Result<Vec<AppManifest>> {
    let steam_root = get_steam_root()?;
    let library_path = steam_root.join("steamapps/libraryfolders.vdf");
    let vdf_data = std::fs::read_to_string(library_path)?;
    let vdf_root = keyvalues_parser::Vdf::parse(&vdf_data)?;

    let Some(entries) = parse_libraryfolders(&vdf_root) else {
        anyhow::bail!("Failed to parse libraryfolders");
    };

    let mut games: Vec<AppManifest> = entries
        .iter()
        .filter_map(|entry| {
            let manifest_path = PathBuf::from(&entry.root_path)
                .join(format!("steamapps/appmanifest_{}.acf", entry.app_id));

            let Ok(vdf_data) = std::fs::read_to_string(&manifest_path) else {
                return None;
            };

            let Ok(vdf_root) = keyvalues_parser::Vdf::parse(&vdf_data) else {
                return None;
            };

            let app_state = vdf_root.value.get_obj()?;
            let name = parse_app_name(app_state)?;
            let name = String::from(name);

            let game_path =
                PathBuf::from(&entry.root_path).join(format!("steamapps/common/{}", name));

            if let Some(actions_path) = find_actions_json(&game_path) {
                Some(AppManifest {
                    name,
                    actions_json_path: Some(actions_path),
                    game_folder: game_path,
                })
            } else {
                None
            }
        })
        .collect();

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(games)
}
