use crate::input_profiles::INPUT_PROFILES_BYTES;
use crate::screens::BindingSourceSelection;
use crate::sources::{self, BindingSourceKind};
use crate::types::manifest::{ActionManifest, InputProfile};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct ControllerTypeScreen {
    controllers: Vec<String>,
    binding_availability: BTreeMap<String, BindingAvailability>,
    list_state: ListState,
    binding_select_state: ListState,
    selected_controller: Option<String>,
    loading: bool,
    error: Option<String>,
    actions_json_path: Option<PathBuf>,
    game_folder: Option<PathBuf>,
    game_name: String,
}

#[derive(Debug, Clone)]
struct BindingAvailability {
    has_default: bool,
    has_xrizer: bool,
    has_vapor: bool,
    has_open_composite: bool,
}

impl BindingAvailability {
    fn new() -> Self {
        Self {
            has_default: false,
            has_xrizer: false,
            has_vapor: false,
            has_open_composite: false,
        }
    }

    fn has_any(&self) -> bool {
        self.has_default || self.has_xrizer || self.has_vapor || self.has_open_composite
    }
}

impl ControllerTypeScreen {
    pub fn new(actions_json_path: PathBuf, game_folder: PathBuf, game_name: String) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut screen = Self {
            controllers: Vec::new(),
            binding_availability: BTreeMap::new(),
            list_state,
            binding_select_state: ListState::default(),
            selected_controller: None,
            loading: true,
            error: None,
            actions_json_path: Some(actions_json_path.clone()),
            game_folder: Some(game_folder),
            game_name,
        };
        screen.load_controllers(actions_json_path);
        screen
    }

    pub fn selected_actions_json_path(&self) -> Option<&PathBuf> {
        self.actions_json_path.as_ref()
    }

    pub fn game_name(&self) -> &str {
        &self.game_name
    }

    pub fn game_folder(&self) -> Option<&PathBuf> {
        self.game_folder.as_ref()
    }

    fn load_controllers(&mut self, actions_json_path: PathBuf) {
        let mut binding_availability: BTreeMap<String, BindingAvailability> = BTreeMap::new();

        for (name, _) in INPUT_PROFILES_BYTES {
            binding_availability.insert((*name).to_string(), BindingAvailability::new());
        }

        if let Ok(content) = std::fs::read_to_string(&actions_json_path) {
            if let Ok(manifest) = serde_json::from_str::<ActionManifest>(&content) {
                for binding in manifest.default_bindings {
                    if let Some(avail) = binding_availability.get_mut(&binding.controller_type) {
                        avail.has_default = true;
                    }
                }
            }
        }

        if let Some(ref game_folder) = self.game_folder {
            let discovered = sources::scan_game_folder(game_folder);
            for db in discovered {
                if let Some(avail) = binding_availability.get_mut(&db.controller_type) {
                    match db.kind {
                        BindingSourceKind::Xrizer => avail.has_xrizer = true,
                        BindingSourceKind::Vapor => avail.has_vapor = true,
                        BindingSourceKind::OpenComposite => avail.has_open_composite = true,
                    }
                }
            }
        }

        self.controllers = INPUT_PROFILES_BYTES
            .iter()
            .map(|(name, _)| (*name).to_string())
            .collect();
        self.binding_availability = binding_availability;
        self.loading = false;

        if self.controllers.is_empty() {
            self.error = Some("No controller types available".to_string());
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> crate::screens::ScreenAction {
        if self.loading || self.controllers.is_empty() {
            return crate::screens::ScreenAction::None;
        }

        if let Some(ref _controller) = self.selected_controller {
            match key {
                KeyCode::Esc => {
                    self.selected_controller = None;
                    self.list_state.select(Some(0));
                    return crate::screens::ScreenAction::None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.binding_select_state.select_previous();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.binding_select_state.select_next();
                }
                KeyCode::PageUp => {
                    self.binding_select_state.select_first();
                }
                KeyCode::PageDown => {
                    self.binding_select_state.select_last();
                }
                KeyCode::Enter => {
                    let controller_type = self.selected_controller.clone().unwrap_or_default();
                    let binding_source = self.selected_binding_source();
                    return crate::screens::ScreenAction::ControllerSelected {
                        controller_type,
                        binding_source,
                    };
                }
                _ => {}
            }
            return crate::screens::ScreenAction::None;
        }

        match key {
            KeyCode::Esc => return crate::screens::ScreenAction::GoBack,
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
                self.selected_controller = self.selected_controller().map(|s| s.to_string());
                if let Some(controller) = self.selected_controller.clone() {
                    self.build_binding_options(&controller);
                    self.binding_select_state.select(Some(0));
                }
            }
            _ => {}
        }

        crate::screens::ScreenAction::None
    }

    fn build_binding_options(&mut self, _controller: &str) {
        self.binding_select_state = ListState::default();
        self.binding_select_state.select(Some(0));
    }

    fn selected_binding_source(&self) -> BindingSourceSelection {
        if let Some(ref controller) = self.selected_controller {
            let avail = self
                .binding_availability
                .get(controller.as_str())
                .cloned()
                .unwrap_or_else(BindingAvailability::new);
            let selected = self.binding_select_state.selected().unwrap_or(0);
            let mut idx = 0;

            if avail.has_default && idx == selected {
                return BindingSourceSelection::Default;
            }
            if avail.has_default {
                idx += 1;
            }

            if avail.has_xrizer && idx == selected {
                return BindingSourceSelection::Xrizer;
            }
            if avail.has_xrizer {
                idx += 1;
            }

            if avail.has_vapor && idx == selected {
                return BindingSourceSelection::Vapor;
            }
            if avail.has_vapor {
                idx += 1;
            }

            if avail.has_open_composite && idx == selected {
                return BindingSourceSelection::OpenComposite;
            }
            if avail.has_open_composite {
                idx += 1;
            }

            BindingSourceSelection::Scratch
        } else {
            BindingSourceSelection::Scratch
        }
    }

    pub fn selected_controller(&self) -> Option<&str> {
        self.list_state
            .selected()
            .map(|i| self.controllers[i].as_str())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if self.loading {
            let title = Paragraph::new(Line::from(Span::styled(
                format!("Select controller profile for {}", self.game_name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);
            frame.render_widget(title, chunks[0]);

            let loading =
                Paragraph::new("Loading controller types...").alignment(Alignment::Center);
            frame.render_widget(loading, chunks[1]);
            return;
        }

        if let Some(ref err) = self.error {
            let title = Paragraph::new(Line::from(Span::styled(
                format!("Select controller profile for {}", self.game_name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);
            frame.render_widget(title, chunks[0]);

            let error_text = Paragraph::new(format!("Error: {}", err))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_text, chunks[1]);
            return;
        }

        if let Some(controller) = self.selected_controller.clone() {
            self.render_binding_selection(frame, area, &controller);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Paragraph::new(Line::from(Span::styled(
            format!("Select controller profile for {}", self.game_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        let block = Block::default()
            .title("Controllers")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let items: Vec<ListItem<'static>> = self
            .controllers
            .iter()
            .map(|controller| {
                let avail = self
                    .binding_availability
                    .get(controller.as_str())
                    .cloned()
                    .unwrap_or_else(BindingAvailability::new);
                let all_gray = !avail.has_any();

                let green = if all_gray {
                    Color::DarkGray
                } else {
                    Color::Green
                };
                let gray = Color::DarkGray;

                let bindings_line = Line::from(vec![
                    Span::styled(
                        format!("{} ", if avail.has_default { "✓" } else { " " }),
                        Style::default().fg(if avail.has_default { green } else { gray }),
                    ),
                    Span::styled(
                        "Default ",
                        Style::default().fg(if avail.has_default { green } else { gray }),
                    ),
                    Span::styled(
                        format!("{} ", if avail.has_xrizer { "✓" } else { " " }),
                        Style::default().fg(if avail.has_xrizer { green } else { gray }),
                    ),
                    Span::styled(
                        "xrizer ",
                        Style::default().fg(if avail.has_xrizer { green } else { gray }),
                    ),
                    Span::styled(
                        format!("{} ", if avail.has_vapor { "✓" } else { " " }),
                        Style::default().fg(if avail.has_vapor { green } else { gray }),
                    ),
                    Span::styled(
                        "VapoR ",
                        Style::default().fg(if avail.has_vapor { green } else { gray }),
                    ),
                    Span::styled(
                        format!("{} ", if avail.has_open_composite { "✓" } else { " " }),
                        Style::default().fg(if avail.has_open_composite {
                            green
                        } else {
                            gray
                        }),
                    ),
                    Span::styled(
                        "OpenComposite",
                        Style::default().fg(if avail.has_open_composite {
                            green
                        } else {
                            gray
                        }),
                    ),
                ]);

                let mut lines = vec![Line::from(vec![Span::styled(
                    InputProfile::format_name_str(controller),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )])];

                lines.push(bindings_line);

                ListItem::new(lines)
            })
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

    fn render_binding_selection(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        controller: &str,
    ) {
        let avail = self
            .binding_availability
            .get(controller)
            .cloned()
            .unwrap_or_else(BindingAvailability::new);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Paragraph::new(Line::from(Span::styled(
            format!(
                "Choose bindings to load for {}",
                InputProfile::format_name_str(controller)
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        let mut items = Vec::new();

        if avail.has_default {
            items.push(ListItem::new(Span::raw("Game default bindings")));
        }
        if avail.has_xrizer {
            items.push(ListItem::new(Span::raw("xrizer bindings")));
        }
        if avail.has_vapor {
            items.push(ListItem::new(Span::raw("VapoR bindings")));
        }
        if avail.has_open_composite {
            items.push(ListItem::new(Span::raw("OpenComposite bindings")));
        }
        items.push(ListItem::new(Span::raw("Start from scratch")));

        let block = Block::default()
            .title("Bindings to load")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );

        frame.render_stateful_widget(list, chunks[1], &mut self.binding_select_state);

        let help = Paragraph::new("↑/↓ navigate • Enter select • Esc back • q quit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }
}
