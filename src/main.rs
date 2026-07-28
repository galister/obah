mod input_profiles;
mod screens;
mod sources;
mod steam;
mod types;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::path::Path;
use std::time::Duration;

use screens::{
    Screen, ScreenAction, BindingSourceSelection, controller_type::ControllerTypeScreen, edit_bindings::EditBindingsScreen,
    game_select::GameSelectScreen,
};
use steam::AppManifest;
use sources::BindingSourceKind;
use std::path::PathBuf;

fn resolve_binding_path(
    game_folder: &Path,
    controller_type: &str,
    binding_source: &BindingSourceSelection,
    actions_json_path: &Path,
) -> Option<PathBuf> {
    match binding_source {
        BindingSourceSelection::Default => {
            let content = std::fs::read_to_string(actions_json_path).ok()?;
            let manifest: types::manifest::ActionManifest = serde_json::from_str(&content).ok()?;
            let binding = manifest.default_bindings.iter().find(|b| b.controller_type == controller_type)?;
            let path = actions_json_path.parent()?.join(&binding.binding_url);
            path.canonicalize().ok()
        }
        BindingSourceSelection::Xrizer => Some(
            BindingSourceKind::Xrizer.save_path(game_folder, controller_type).canonicalize().ok()?,
        ),
        BindingSourceSelection::Vapor => Some(
            BindingSourceKind::Vapor.save_path(game_folder, controller_type).canonicalize().ok()?,
        ),
        BindingSourceSelection::OpenComposite => Some(
            BindingSourceKind::OpenComposite.save_path(game_folder, controller_type)
                .canonicalize()
                .ok()?,
        ),
        BindingSourceSelection::Scratch => None,
    }
}

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let args: Vec<String> = std::env::args().collect();

    let mut game_select: Option<GameSelectScreen> = None;
    let mut load_rx: Option<std::sync::mpsc::Receiver<Result<Vec<AppManifest>, String>>> = None;
    let mut controller_type: Option<ControllerTypeScreen> = None;
    let mut edit_bindings: Option<EditBindingsScreen> = None;
    let mut current_screen = Screen::GameSelect;

    if args.len() > 1 {
        let provided_path = Path::new(&args[1]);
        let resolved_path = if provided_path.is_absolute() {
            provided_path.to_path_buf()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(provided_path))
                .unwrap_or_else(|| provided_path.to_path_buf())
        };

        if !resolved_path.is_dir() {
            eprintln!("Error: '{}' is not a directory", args[1]);
            std::process::exit(1);
        }

        if let Some(actions_json) = steam::find_actions_json(&resolved_path) {
            let game_name = resolved_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&args[1])
                .to_string();
            controller_type = Some(ControllerTypeScreen::new(actions_json, resolved_path, game_name));
            current_screen = Screen::ControllerType;
        } else {
            eprintln!("Error: no actions.json found in '{}'", args[1]);
            std::process::exit(1);
        }
    } else {
        let (gs, rx) = GameSelectScreen::start_loading();
        game_select = Some(gs);
        load_rx = Some(rx);
    }

    'main: loop {
        if let (Some(gs), Some(rx)) = (&mut game_select, &load_rx) {
            gs.check_loading(rx);
        }

        terminal.draw(|f| match current_screen {
            Screen::GameSelect => {
                if let Some(ref mut gs) = game_select {
                    gs.render(f);
                }
            }
            Screen::ControllerType => {
                if let Some(ref mut ct) = controller_type {
                    ct.render(f);
                }
            }
            Screen::EditBindings => {
                if let Some(ref mut eb) = edit_bindings {
                    eb.render(f);
                }
            }
        })?;

        while event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    if key.code == KeyCode::Char('q') && current_screen != Screen::EditBindings {
                        break 'main;
                    }

                    match current_screen {
                        Screen::GameSelect => {
                            if let Some(gs) = &mut game_select {
                                gs.handle_key(key.code);

                                if key.code == KeyCode::Enter && gs.should_transition() {
                            if let (Some(actions_path), Some(game_folder), Some(game_name)) = (
                                gs.selected_actions_json_path(),
                                gs.selected_game_folder(),
                                gs.selected_game_name(),
                            ) {
                                controller_type =
                                    Some(ControllerTypeScreen::new(actions_path, game_folder, game_name));
                                current_screen = Screen::ControllerType;
                                    }
                                }
                            }
                        }
                        Screen::ControllerType => {
                            if let Some(ref mut ct) = controller_type {
                                match ct.handle_key(key.code) {
                                    ScreenAction::GoBack => {
                                        controller_type = None;
                                        current_screen = Screen::GameSelect;
                                    }
                                    ScreenAction::ControllerSelected { controller_type: ct_name, binding_source } => {
                                        let game_name = ct.game_name().to_string();
                                        let game_folder = ct.game_folder().cloned().unwrap_or_default();
                                        let actions_path = ct
                                            .selected_actions_json_path()
                                            .cloned()
                                            .unwrap_or_default();

                                        let binding_json_path =
                                            resolve_binding_path(&game_folder, &ct_name, &binding_source, &actions_path);

                                        edit_bindings = Some(EditBindingsScreen::new_with_binding(
                                            ct_name,
                                            game_name,
                                            game_folder,
                                            actions_path,
                                            binding_json_path,
                                        ));
                                        current_screen = Screen::EditBindings;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Screen::EditBindings => {
                            if let Some(ref mut eb) = edit_bindings {
                                 match eb.handle_key(key.code) {
                                     ScreenAction::GoBack => {
                                         edit_bindings = None;
                                         current_screen = Screen::ControllerType;
                                     }
                                      ScreenAction::Quit => {
                                          break 'main;
                                     }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
