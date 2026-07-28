use crate::steam::AppManifest;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Debug)]
pub struct GameSelectScreen {
    games: Vec<AppManifest>,
    list_state: ListState,
    loading: bool,
    error: Option<String>,
    browse_path: Option<PathBuf>,
}

impl GameSelectScreen {
    pub fn start_loading() -> (Self, mpsc::Receiver<Result<Vec<AppManifest>, String>>) {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = crate::steam::list_steamvr_games().map_err(|e| e.to_string());
            let _ = tx.send(result);
        });

        let mut screen = Self {
            games: Vec::new(),
            list_state: ListState::default(),
            loading: true,
            error: None,
            browse_path: None,
        };
        screen.list_state.select(Some(0));

        (screen, rx)
    }

    pub fn check_loading(&mut self, rx: &mpsc::Receiver<Result<Vec<AppManifest>, String>>) {
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(games) => {
                    self.games = games;
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e);
                }
            }
            self.loading = false;
        }
    }

    pub fn selected_actions_json_path(&self) -> Option<PathBuf> {
        let Some(selected_index) = self.list_state.selected() else {
            return None;
        };

        let use_current_index = self.games.len();
        let browse_index = self.games.len() + 1;

        if selected_index < self.games.len() {
            self.games[selected_index].actions_json_path.clone()
        } else if selected_index == use_current_index {
            let cwd = std::env::current_dir().ok()?;
            crate::steam::find_actions_json(&cwd)
        } else if selected_index == browse_index {
            self.browse_path
                .as_ref()
                .and_then(|p| crate::steam::find_actions_json(p))
        } else {
            None
        }
    }

    pub fn selected_game_folder(&self) -> Option<PathBuf> {
        let Some(selected_index) = self.list_state.selected() else {
            return None;
        };

        let use_current_index = self.games.len();
        let browse_index = self.games.len() + 1;

        if selected_index < self.games.len() {
            Some(self.games[selected_index].game_folder.clone())
        } else if selected_index == use_current_index {
            std::env::current_dir().ok()
        } else if selected_index == browse_index {
            self.browse_path.clone()
        } else {
            None
        }
    }

    pub fn selected_game_name(&self) -> Option<String> {
        let Some(selected_index) = self.list_state.selected() else {
            return None;
        };

        let use_current_index = self.games.len();
        let browse_index = self.games.len() + 1;

        if selected_index < self.games.len() {
            Some(self.games[selected_index].name.clone())
        } else if selected_index == use_current_index {
            let cwd = std::env::current_dir().ok()?;
            cwd.file_name()?.to_str().map(String::from)
        } else if selected_index == browse_index {
            let path = self.browse_path.clone()?;
            path.file_name()?.to_str().map(String::from)
        } else {
            None
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        if self.loading {
            return;
        }

        let use_current_index = self.games.len();
        let browse_index = self.games.len() + 1;
        let total_items = self.games.len() + 2;

        if total_items == 0 {
            return;
        }

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select_next();
            }
            KeyCode::PageUp => {
                self.list_state.select_first();
            }
            KeyCode::PageDown => {
                self.list_state.select_last();
            }
            KeyCode::Enter => {
                let selected_index = self.list_state.selected().unwrap_or_default();
                if selected_index == use_current_index {
                    self.handle_use_current();
                } else if selected_index == browse_index {
                    self.handle_browse();
                }
            }
            _ => {}
        }
    }

    fn handle_use_current(&mut self) {
        let cwd = std::env::current_dir().expect("Could not get current directory");
        println!("Using current directory: {}", cwd.display());
    }

    fn handle_browse(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);

        let result = dialoguer::Input::<String>::new()
            .with_prompt("Enter game folder path")
            .interact_text();

        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen);

        if let Ok(path_str) = result {
            if !path_str.is_empty() {
                let path = std::path::PathBuf::from(path_str);
                if path.exists() {
                    self.browse_path = Some(path);
                }
            }
        }
    }

    pub fn should_transition(&self) -> bool {
        let selected_index = self.list_state.selected().unwrap_or_default();
        selected_index < self.games.len()
            || selected_index == self.games.len()
            || selected_index == self.games.len() + 1
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Paragraph::new(Line::from(Span::styled(
            "Game Select",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        if self.loading {
            let loading = Paragraph::new("Scanning games...").alignment(Alignment::Center);
            frame.render_widget(loading, chunks[1]);
            return;
        }

        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_text, chunks[1]);
            return;
        }

        let block = Block::default()
            .title("Games")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let selected_index = self.list_state.selected().unwrap_or_default();
        let items: Vec<ListItem> = self
            .games
            .iter()
            .enumerate()
            .map(|(i, game)| {
                let style = if i == selected_index {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                ListItem::new(game.name.as_str()).style(style)
            })
            .chain(std::iter::once({
                let style = if selected_index == self.games.len() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new("[+] Use current directory").style(style)
            }))
            .chain(std::iter::once({
                let style = if selected_index == self.games.len() + 1 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new("[+] Browse for game folder").style(style)
            }))
            .collect();

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let help = Paragraph::new("↑/↓ navigate • PgUp/PgDn jump • Enter select • q quit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }
}
