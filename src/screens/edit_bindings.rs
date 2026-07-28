use crate::input_profiles::load_input_profile;
use crate::screens::list_utils;
use crate::sources::BindingSourceKind;
use crate::types::{
    manifest::{
        ActionManifest, ActionSetBindings, ActionSetDefinition, ActionSetUsage, ActionType,
        BindingInput, BindingMode, BindingOutput, BindingUiMode, ChordBinding, ChordInput,
        ChordInputKind, PathBinding, SourceBinding, SteamVrBindingFile,
    },
    profile::{InputProfile, InputSourceDef, InputSourceType, InputSourceValue},
};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
struct SourceInfo {
    path: String,
    name: String,
    def: InputSourceDef,
}

struct SavePathDialog {
    selected: usize,
}

impl SavePathDialog {
    fn new() -> Self {
        Self { selected: 0 }
    }

    fn options(&self) -> Vec<&str> {
        vec!["xrizer", "VapoR", "OpenComposite"]
    }

    fn selected_kind(&self) -> Option<BindingSourceKind> {
        let kinds = [
            BindingSourceKind::Xrizer,
            BindingSourceKind::Vapor,
            BindingSourceKind::OpenComposite,
        ];
        if self.selected < kinds.len() {
            Some(kinds[self.selected].clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathBindingKind {
    Pose,
    Skeleton,
    Haptic,
}

impl PathBindingKind {
    fn display_name(self) -> &'static str {
        match self {
            Self::Pose => "Pose",
            Self::Skeleton => "Skeleton",
            Self::Haptic => "Haptic",
        }
    }

    fn action_type(self) -> ActionType {
        match self {
            Self::Pose => ActionType::Pose,
            Self::Skeleton => ActionType::Skeleton,
            Self::Haptic => ActionType::Vibration,
        }
    }

    fn action_direction(self) -> ActionDirection {
        match self {
            Self::Pose | Self::Skeleton => ActionDirection::Input,
            Self::Haptic => ActionDirection::Output,
        }
    }
}

#[derive(Debug, Clone)]
struct PathSourceInfo {
    /// Full binding path, with the handed user root applied.
    path: String,
    /// Display name derived from the InputProfile path.
    name: String,
    kind: PathBindingKind,
}

#[derive(Debug, Clone)]
struct ChordSourceInfo {
    /// Full binding path, with the handed user root applied.
    path: String,
    /// Display name derived from the InputProfile path.
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControllerSide {
    Left,
    Right,
    Single,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelFocus {
    Left,
    Right,
    Other,
    Chords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequiredActionType {
    Boolean,
    Vector1,
    Vector2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingModalFocus {
    BindingList,
    Editor,
}

struct ModePicker {
    options: Vec<BindingMode>,
    selected: usize,
}

struct ActionPicker {
    input: BindingInput,
    options: Vec<Option<String>>,
    selected: usize,
}

#[derive(Debug, Clone)]
struct ParameterSpec {
    name: &'static str,
    label: &'static str,
    default_value: String,
    choices: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParameterChoice {
    name: Option<String>,
    label: String,
    default_value: String,
}

struct ParameterPicker {
    options: Vec<ParameterChoice>,
    selected: usize,
}

struct ParameterEditor {
    original_name: Option<String>,
    name: String,
    value: String,
    selected_field: usize,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum SourceEditorField {
    Mode,
    Input(BindingInput),
    Parameter(String),
    AddParameter,
}

struct SourceBindingModal {
    action_set_name: String,
    source: SourceInfo,
    side: ControllerSide,
    new_binding_path: String,
    drafts: Vec<SourceBinding>,
    selected_binding: usize,
    selected_field: usize,
    focus: BindingModalFocus,
    mode_picker: Option<ModePicker>,
    action_picker: Option<ActionPicker>,
    parameter_picker: Option<ParameterPicker>,
    parameter_editor: Option<ParameterEditor>,
}

#[derive(Debug, Clone)]
struct StringChoice {
    value: String,
    label: String,
}

struct StringPicker {
    title: String,
    options: Vec<StringChoice>,
    selected: usize,
}

struct ChordKindPicker {
    input_index: usize,
    options: Vec<ChordInputKind>,
    selected: usize,
}

#[derive(Debug, Clone)]
enum OtherBindingEntry {
    Path { kind: PathBindingKind, index: usize },
    AddPath(PathBindingKind),
    Chord { index: usize },
    AddChord,
}

struct PathBindingModal {
    action_set_name: String,
    kind: PathBindingKind,
    original_index: Option<usize>,
    draft: PathBinding,
    selected_field: usize,
    source_picker: Option<StringPicker>,
    action_picker: Option<StringPicker>,
    error: Option<String>,
}

struct ChordBindingModal {
    action_set_name: String,
    original_index: Option<usize>,
    draft: ChordBinding,
    selected_field: usize,
    action_picker: Option<StringPicker>,
    source_picker: Option<(usize, StringPicker)>,
    kind_picker: Option<ChordKindPicker>,
    error: Option<String>,
}

enum OtherBindingModal {
    Path(PathBindingModal),
    Chord(ChordBindingModal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalOutcome {
    Continue,
    Apply,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OtherModalOutcome {
    Continue,
    Apply,
    Delete,
    Cancel,
}

pub struct EditBindingsScreen {
    controller_type: String,
    controller_display_name: String,
    game_name: String,
    game_folder: PathBuf,
    ui_mode: BindingUiMode,
    action_sets: Vec<ActionSetDefinition>,
    actions: BTreeMap<String, ActionType>,
    binding: SteamVrBindingFile,
    binding_label: String,
    binding_path: Option<PathBuf>,
    save_path_dialog: Option<SavePathDialog>,

    selected_action_set_index: usize,
    left_sources: Vec<SourceInfo>,
    right_sources: Vec<SourceInfo>,
    path_sources: Vec<PathSourceInfo>,
    chord_sources: Vec<ChordSourceInfo>,
    left_list_state: ListState,
    right_list_state: ListState,
    other_list_state: ListState,
    chord_list_state: ListState,
    panel_focus: PanelFocus,
    binding_modal: Option<SourceBindingModal>,
    other_modal: Option<OtherBindingModal>,
    dirty: bool,

    loading: bool,
    error: Option<String>,
    discard_dialog: bool,
    quit_on_discard: bool,
    status_message: Option<String>,
}

impl EditBindingsScreen {
    pub fn new_with_binding(
        controller_type: String,
        game_name: String,
        game_folder: PathBuf,
        actions_json_path: PathBuf,
        binding_json_path: Option<PathBuf>,
    ) -> Self {
        Self::new_inner(
            controller_type,
            game_name,
            game_folder,
            actions_json_path,
            binding_json_path,
        )
    }

    fn new_inner(
        controller_type: String,
        game_name: String,
        game_folder: PathBuf,
        actions_json_path: PathBuf,
        binding_json_path: Option<PathBuf>,
    ) -> Self {
        let mut left_list_state = ListState::default();
        left_list_state.select(Some(0));

        let mut right_list_state = ListState::default();
        right_list_state.select(Some(0));

        let mut other_list_state = ListState::default();
        other_list_state.select(Some(0));

        let mut chord_list_state = ListState::default();
        chord_list_state.select(Some(0));

        let controller_display_name =
            crate::types::manifest::InputProfile::format_name_str(&controller_type);

        let mut screen = Self {
            controller_type: controller_type.clone(),
            controller_display_name,
            game_name,
            game_folder,
            ui_mode: BindingUiMode::ControllerHanded,
            action_sets: Vec::new(),
            actions: BTreeMap::new(),
            binding: empty_binding(controller_type),
            binding_label: "no binding file".to_owned(),
            binding_path: None,
            save_path_dialog: None,
            selected_action_set_index: 0,
            left_sources: Vec::new(),
            right_sources: Vec::new(),
            path_sources: Vec::new(),
            chord_sources: Vec::new(),
            left_list_state,
            right_list_state,
            other_list_state,
            chord_list_state,
            panel_focus: PanelFocus::Left,
            binding_modal: None,
            other_modal: None,
            dirty: false,
            loading: true,
            error: None,
            discard_dialog: false,
            quit_on_discard: false,
            status_message: None,
        };

        screen.load_data(actions_json_path, binding_json_path);
        screen
    }

    fn load_data(&mut self, actions_json_path: PathBuf, binding_json_path: Option<PathBuf>) {
        let content = match std::fs::read_to_string(&actions_json_path) {
            Ok(content) => content,
            Err(error) => {
                self.fail(format!("Failed to read actions.json: {error}"));
                return;
            }
        };

        let manifest: ActionManifest = match serde_json::from_str(&content) {
            Ok(manifest) => manifest,
            Err(error) => {
                self.fail(format!("Failed to parse actions.json: {error}"));
                return;
            }
        };

        if manifest.action_sets.is_empty() {
            self.fail("No action sets found in actions.json".to_owned());
            return;
        }

        self.actions = manifest
            .actions
            .iter()
            .map(|action| (action.name.clone(), action.kind.clone()))
            .collect();
        self.action_sets = manifest.action_sets.clone();

        let loaded_binding = match binding_json_path {
            Some(path) => load_binding_file(path).map(Some),
            None => load_manifest_binding(&manifest, &self.controller_type, &actions_json_path),
        };

        match loaded_binding {
            Ok(Some((binding, path))) => {
                self.binding = binding;
                // Always prompt for save location; don't overwrite defaults automatically.
                self.binding_label = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("default binding")
                    .to_owned();
            }
            Ok(None) => {
                // It is still useful to show the profile and permit adding a new binding.
                self.binding = empty_binding(self.controller_type.clone());
                self.binding_label = "new binding (no default for this controller)".to_owned();
            }
            Err(error) => {
                self.fail(error);
                return;
            }
        }

        let profile = match load_input_profile(&self.controller_type) {
            Some(profile) => profile,
            None => {
                self.fail(format!(
                    "No input profile found for controller type: {}",
                    self.controller_type
                ));
                return;
            }
        };

        self.ui_mode = profile.input_bindingui_mode.clone();
        self.load_profile_sources(&profile);
        self.loading = false;

        list_utils::apply_selection(&mut self.left_list_state, 0, self.left_sources.len());
        list_utils::apply_selection(&mut self.right_list_state, 0, self.right_sources.len());
        let other_len = self.other_path_entries().len();
        list_utils::apply_selection(&mut self.other_list_state, 0, other_len);
        let chord_len = self.chord_binding_entries().len();
        list_utils::apply_selection(&mut self.chord_list_state, 0, chord_len);
    }

    fn load_profile_sources(&mut self, profile: &InputProfile) {
        let mut left_sources = Vec::new();
        let mut right_sources = Vec::new();
        let mut path_sources = Vec::new();
        let mut chord_sources = Vec::new();

        for (path, value) in &profile.input_source {
            match value {
                InputSourceValue::Def(def) => {
                    collect_source(
                        path,
                        def,
                        def.side.as_deref(),
                        &self.ui_mode,
                        &mut left_sources,
                        &mut right_sources,
                    );
                    collect_aux_sources(
                        path,
                        def,
                        def.side.as_deref(),
                        &self.ui_mode,
                        &self.binding,
                        &mut path_sources,
                        &mut chord_sources,
                    );
                }
                InputSourceValue::MapInputSource(definitions) => {
                    for (map_key, def) in definitions {
                        let mapped_side = side_from_map_key(map_key);
                        let side = def.side.as_deref().or(mapped_side);
                        let mapped_path = if mapped_side.is_some() {
                            path.clone()
                        } else if map_key.starts_with('/') {
                            map_key.clone()
                        } else {
                            format!(
                                "{}/{}",
                                path.trim_end_matches('/'),
                                map_key.trim_start_matches('/')
                            )
                        };

                        collect_source(
                            &mapped_path,
                            def,
                            side,
                            &self.ui_mode,
                            &mut left_sources,
                            &mut right_sources,
                        );
                        collect_aux_sources(
                            &mapped_path,
                            def,
                            side,
                            &self.ui_mode,
                            &self.binding,
                            &mut path_sources,
                            &mut chord_sources,
                        );
                    }
                }
            }
        }

        let sort_sources = |a: &SourceInfo, b: &SourceInfo| {
            a.def
                .order
                .cmp(&b.def.order)
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.path.cmp(&b.path))
        };

        left_sources.sort_by(sort_sources);
        right_sources.sort_by(sort_sources);
        path_sources.sort_by(|a, b| {
            path_binding_kind_order(a.kind)
                .cmp(&path_binding_kind_order(b.kind))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.path.cmp(&b.path))
        });
        chord_sources.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
        dedup_path_sources(&mut path_sources);
        dedup_chord_sources(&mut chord_sources);

        self.left_sources = left_sources;
        self.right_sources = right_sources;
        self.path_sources = path_sources;
        self.chord_sources = chord_sources;
    }

    fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
    }

    pub fn handle_key(&mut self, key: KeyCode) -> crate::screens::ScreenAction {
        if self.loading || self.error.is_some() {
            return crate::screens::ScreenAction::None;
        }

        if self.discard_dialog {
            return self.handle_discard_dialog_key(key);
        }

        if self.save_path_dialog.is_some() {
            return self.handle_save_path_dialog_key(key);
        }

        if self.binding_modal.is_some() {
            self.handle_binding_modal_key(key);
            return crate::screens::ScreenAction::None;
        }

        if self.other_modal.is_some() {
            self.handle_other_modal_key(key);
            return crate::screens::ScreenAction::None;
        }

        self.status_message = None;

        match key {
            KeyCode::Esc => {
                if self.dirty {
                    self.discard_dialog = true;
                    self.quit_on_discard = false;
                } else {
                    return crate::screens::ScreenAction::GoBack;
                }
            }
            KeyCode::Char('q') => {
                if self.dirty {
                    self.discard_dialog = true;
                    self.quit_on_discard = true;
                } else {
                    return crate::screens::ScreenAction::Quit;
                }
            }
            KeyCode::Enter => match self.panel_focus {
                PanelFocus::Left | PanelFocus::Right => self.open_selected_source_modal(),
                PanelFocus::Other => self.open_selected_other_modal(),
                PanelFocus::Chords => self.open_selected_other_modal(),
            },
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
            KeyCode::Left | KeyCode::Char('h') => self.move_action_set_left(),
            KeyCode::Right | KeyCode::Char('l') => self.move_action_set_right(),
            KeyCode::PageUp => self.move_to_first(),
            KeyCode::PageDown => self.move_to_last(),
            KeyCode::Tab => self.switch_panel(),
            KeyCode::Char('1')
            | KeyCode::Char('2')
            | KeyCode::Char('3')
            | KeyCode::Char('4')
            | KeyCode::Char('5')
            | KeyCode::Char('6')
            | KeyCode::Char('7')
            | KeyCode::Char('8')
            | KeyCode::Char('9') => self.switch_action_set_by_key(key),
            KeyCode::Char('w') => self.save_binding(),
            _ => {}
        }

        crate::screens::ScreenAction::None
    }

    fn handle_discard_dialog_key(&mut self, key: KeyCode) -> crate::screens::ScreenAction {
        match key {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.dirty = false;
                self.discard_dialog = false;
                if self.quit_on_discard {
                    self.quit_on_discard = false;
                    crate::screens::ScreenAction::Quit
                } else {
                    crate::screens::ScreenAction::GoBack
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.discard_dialog = false;
                crate::screens::ScreenAction::None
            }
            _ => crate::screens::ScreenAction::None,
        }
    }

    fn handle_save_path_dialog_key(&mut self, key: KeyCode) -> crate::screens::ScreenAction {
        let Some(ref mut dialog) = self.save_path_dialog else {
            return crate::screens::ScreenAction::None;
        };

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if dialog.selected > 0 {
                    dialog.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if dialog.selected < dialog.options().len() - 1 {
                    dialog.selected += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(kind) = dialog.selected_kind() {
                    let path = kind.save_path(&self.game_folder, &self.controller_type);
                    self.save_path_dialog = None;
                    if let Err(error) = self.save_binding_to(&path) {
                        self.status_message = Some(error);
                    } else {
                        self.status_message = Some("Saved.".to_owned());
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.save_path_dialog = None;
            }
            _ => {}
        }

        crate::screens::ScreenAction::None
    }

    fn open_selected_source_modal(&mut self) {
        let (source, side) = match self.panel_focus {
            PanelFocus::Left => {
                let Some(index) = self.left_list_state.selected() else {
                    return;
                };
                let Some(source) = self.left_sources.get(index).cloned() else {
                    return;
                };
                let side = if self.has_two_panels() {
                    ControllerSide::Left
                } else {
                    ControllerSide::Single
                };
                (source, side)
            }
            PanelFocus::Right => {
                let Some(index) = self.right_list_state.selected() else {
                    return;
                };
                let Some(source) = self.right_sources.get(index).cloned() else {
                    return;
                };
                (source, ControllerSide::Right)
            }
            PanelFocus::Other => return,
            PanelFocus::Chords => return,
        };

        let Some(action_set) = self.action_sets.get(self.selected_action_set_index) else {
            return;
        };
        let action_set_name = action_set.name.clone();

        let drafts = find_action_set_bindings(&self.binding.bindings, &action_set_name)
            .map(ActionSetBindings::sources)
            .unwrap_or_default()
            .iter()
            .filter(|binding| source_binding_matches(&binding.path, &source.path, side))
            .cloned()
            .collect();

        let new_binding_path =
            default_binding_path(&source.path, side, &self.ui_mode, &self.binding);

        self.binding_modal = Some(SourceBindingModal::new(
            action_set_name,
            source,
            side,
            new_binding_path,
            drafts,
        ));
    }

    fn handle_binding_modal_key(&mut self, key: KeyCode) {
        let Some(mut modal) = self.binding_modal.take() else {
            return;
        };

        match modal.handle_key(key, &self.actions) {
            ModalOutcome::Continue => self.binding_modal = Some(modal),
            ModalOutcome::Cancel => {}
            ModalOutcome::Apply => {
                self.apply_binding_modal(modal);
                self.dirty = true;
            }
        }
    }

    fn apply_binding_modal(&mut self, modal: SourceBindingModal) {
        let action_set_name =
            resolved_action_set_name(&self.binding.bindings, &modal.action_set_name);
        let action_set = self
            .binding
            .bindings
            .entry(action_set_name)
            .or_default();
        let sources = action_set.sources_mut();

        let insert_at = sources
            .iter()
            .position(|binding| {
                source_binding_matches(&binding.path, &modal.source.path, modal.side)
            })
            .unwrap_or(sources.len());

        sources.retain(|binding| {
            !source_binding_matches(&binding.path, &modal.source.path, modal.side)
        });

        for (offset, binding) in modal.drafts.into_iter().enumerate() {
            sources.insert(insert_at + offset, binding);
        }
    }

    fn other_path_entries(&self) -> Vec<OtherBindingEntry> {
        let empty = ActionSetBindings::default();
        let bindings = self.current_action_set_bindings().unwrap_or(&empty);
        let mut entries = Vec::new();

        for index in 0..bindings.poses.as_deref().unwrap_or_default().len() {
            entries.push(OtherBindingEntry::Path {
                kind: PathBindingKind::Pose,
                index,
            });
        }
        if self.can_add_path_binding(PathBindingKind::Pose) {
            entries.push(OtherBindingEntry::AddPath(PathBindingKind::Pose));
        }

        for index in 0..bindings.skeleton.as_deref().unwrap_or_default().len() {
            entries.push(OtherBindingEntry::Path {
                kind: PathBindingKind::Skeleton,
                index,
            });
        }
        if self.can_add_path_binding(PathBindingKind::Skeleton) {
            entries.push(OtherBindingEntry::AddPath(PathBindingKind::Skeleton));
        }

        for index in 0..bindings.haptics.as_deref().unwrap_or_default().len() {
            entries.push(OtherBindingEntry::Path {
                kind: PathBindingKind::Haptic,
                index,
            });
        }
        if self.can_add_path_binding(PathBindingKind::Haptic) {
            entries.push(OtherBindingEntry::AddPath(PathBindingKind::Haptic));
        }

        entries
    }

    fn chord_binding_entries(&self) -> Vec<OtherBindingEntry> {
        let empty = ActionSetBindings::default();
        let bindings = self.current_action_set_bindings().unwrap_or(&empty);
        let mut entries = Vec::new();

        for index in 0..bindings.chords.as_deref().unwrap_or_default().len() {
            entries.push(OtherBindingEntry::Chord { index });
        }
        if !self.chord_sources.is_empty()
            && self.has_compatible_action(&ActionType::Boolean, ActionDirection::Input)
        {
            entries.push(OtherBindingEntry::AddChord);
        }

        entries
    }

    fn can_add_path_binding(&self, kind: PathBindingKind) -> bool {
        self.path_sources.iter().any(|source| source.kind == kind)
            && self.has_compatible_action(&kind.action_type(), kind.action_direction())
    }

    fn has_compatible_action(
        &self,
        required_type: &ActionType,
        direction: ActionDirection,
    ) -> bool {
        let Some(action_set) = self.action_sets.get(self.selected_action_set_index) else {
            return false;
        };
        !action_choices(
            &self.actions,
            &action_set.name,
            required_type,
            direction,
            None,
        )
        .is_empty()
    }

    fn open_selected_other_modal(&mut self) {
        let Some(action_set) = self.action_sets.get(self.selected_action_set_index) else {
            return;
        };
        let action_set_name = action_set.name.clone();
        let bindings = self
            .current_action_set_bindings()
            .cloned()
            .unwrap_or_default();

        match self.panel_focus {
            PanelFocus::Other => self.open_selected_path_binding_modal(&bindings, action_set_name),
            PanelFocus::Chords => {
                self.open_selected_chord_binding_modal(&bindings, action_set_name)
            }
            _ => {}
        }
    }

    fn open_selected_path_binding_modal(
        &mut self,
        bindings: &ActionSetBindings,
        action_set_name: String,
    ) {
        let entries = self.other_path_entries();
        let Some(index) = self.other_list_state.selected() else {
            return;
        };
        let Some(entry) = entries.get(index).cloned() else {
            return;
        };

        self.other_modal = match entry {
            OtherBindingEntry::Path { kind, index } => path_bindings(bindings, kind)
                .get(index)
                .cloned()
                .map(|draft| {
                    OtherBindingModal::Path(PathBindingModal::new(
                        action_set_name,
                        kind,
                        Some(index),
                        draft,
                    ))
                }),
            OtherBindingEntry::AddPath(kind) => {
                let path = self
                    .path_sources
                    .iter()
                    .find(|source| source.kind == kind)
                    .map(|source| source.path.clone())
                    .unwrap_or_default();
                let output = action_choices(
                    &self.actions,
                    &action_set_name,
                    &kind.action_type(),
                    kind.action_direction(),
                    None,
                )
                .into_iter()
                .next()
                .map(|choice| choice.value)
                .unwrap_or_default();
                Some(OtherBindingModal::Path(PathBindingModal::new(
                    action_set_name,
                    kind,
                    None,
                    PathBinding {
                        output,
                        path,
                        requirement: None,
                        extra: BTreeMap::new(),
                    },
                )))
            }
            OtherBindingEntry::Chord { .. } | OtherBindingEntry::AddChord => None,
        };
    }

    fn open_selected_chord_binding_modal(
        &mut self,
        bindings: &ActionSetBindings,
        action_set_name: String,
    ) {
        let entries = self.chord_binding_entries();
        let Some(index) = self.chord_list_state.selected() else {
            return;
        };
        let Some(entry) = entries.get(index).cloned() else {
            return;
        };

        self.other_modal = match entry {
            OtherBindingEntry::Chord { index } => bindings
                .chords
                .as_deref()
                .and_then(|chords| chords.get(index))
                .cloned()
                .map(|draft| {
                    OtherBindingModal::Chord(ChordBindingModal::new(
                        action_set_name,
                        Some(index),
                        draft,
                    ))
                }),
            OtherBindingEntry::AddChord => {
                let output = action_choices(
                    &self.actions,
                    &action_set_name,
                    &ActionType::Boolean,
                    ActionDirection::Input,
                    None,
                )
                .into_iter()
                .next()
                .map(|choice| choice.value)
                .unwrap_or_default();
                let inputs = self
                    .chord_sources
                    .first()
                    .map(|source| vec![ChordInput(source.path.clone(), ChordInputKind::Held)])
                    .unwrap_or_default();
                Some(OtherBindingModal::Chord(ChordBindingModal::new(
                    action_set_name,
                    None,
                    ChordBinding {
                        inputs,
                        output,
                        extra: BTreeMap::new(),
                    },
                )))
            }
            OtherBindingEntry::Path { .. } | OtherBindingEntry::AddPath(_) => None,
        };
    }

    fn handle_other_modal_key(&mut self, key: KeyCode) {
        let Some(mut modal) = self.other_modal.take() else {
            return;
        };

        let outcome = match &mut modal {
            OtherBindingModal::Path(modal) => {
                modal.handle_key(key, &self.actions, &self.path_sources)
            }
            OtherBindingModal::Chord(modal) => {
                modal.handle_key(key, &self.actions, &self.chord_sources)
            }
        };

        match outcome {
            OtherModalOutcome::Continue => self.other_modal = Some(modal),
            OtherModalOutcome::Cancel => {}
            OtherModalOutcome::Apply => {
                self.apply_other_modal(modal, false);
                self.dirty = true;
                self.clamp_other_selection();
            }
            OtherModalOutcome::Delete => {
                self.apply_other_modal(modal, true);
                self.dirty = true;
                self.clamp_other_selection();
            }
        }
    }

    fn apply_other_modal(&mut self, modal: OtherBindingModal, delete: bool) {
        match modal {
            OtherBindingModal::Path(modal) => {
                let action_set_name =
                    resolved_action_set_name(&self.binding.bindings, &modal.action_set_name);
                let action_set = self
                    .binding
                    .bindings
                    .entry(action_set_name)
                    .or_default();
                let bindings = path_bindings_mut(action_set, modal.kind);
                match modal.original_index {
                    Some(index) if index < bindings.len() => {
                        if delete {
                            bindings.remove(index);
                        } else {
                            bindings[index] = modal.draft;
                        }
                    }
                    None if !delete => bindings.push(modal.draft),
                    _ => {}
                }
            }
            OtherBindingModal::Chord(modal) => {
                let action_set_name =
                    resolved_action_set_name(&self.binding.bindings, &modal.action_set_name);
                let action_set = self
                    .binding
                    .bindings
                    .entry(action_set_name)
                    .or_default();
                let bindings = action_set.chords_mut();
                match modal.original_index {
                    Some(index) if index < bindings.len() => {
                        if delete {
                            bindings.remove(index);
                        } else {
                            bindings[index] = modal.draft;
                        }
                    }
                    None if !delete => bindings.push(modal.draft),
                    _ => {}
                }
            }
        }
    }

    fn clamp_other_selection(&mut self) {
        let len = self.other_path_entries().len();
        let selected = self.other_list_state.selected().unwrap_or(0);
        list_utils::apply_selection(&mut self.other_list_state, selected, len);

        let chord_len = self.chord_binding_entries().len();
        let chord_selected = self.chord_list_state.selected().unwrap_or(0);
        list_utils::apply_selection(&mut self.chord_list_state, chord_selected, chord_len);
    }

    pub fn binding(&self) -> &SteamVrBindingFile {
        &self.binding
    }

    pub fn binding_mut(&mut self) -> &mut SteamVrBindingFile {
        self.dirty = true;
        &mut self.binding
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn save_binding(&mut self) {
        if let Some(path) = self.binding_path.clone() {
            if let Err(error) = self.save_binding_to(&path) {
                self.status_message = Some(error);
            } else {
                self.status_message = Some("Saved.".to_owned());
            }
        } else {
            self.save_path_dialog = Some(SavePathDialog::new());
        }
    }

    pub fn save_binding_to(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("Failed to create directory {}: {error}", parent.display())
            })?;
        }

        let json = serde_json::to_string_pretty(&self.binding)
            .map_err(|error| format!("Failed to serialize binding: {error}"))?;
        std::fs::write(path, json)
            .map_err(|error| format!("Failed to write binding {}: {error}", path.display()))?;

        self.binding_path = Some(path.to_path_buf());
        self.binding_label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("binding file")
            .to_owned();
        self.dirty = false;
        Ok(())
    }

    fn has_two_panels(&self) -> bool {
        matches!(&self.ui_mode, BindingUiMode::ControllerHanded)
    }

    fn active_panel(&mut self) -> &mut ListState {
        match self.panel_focus {
            PanelFocus::Left => &mut self.left_list_state,
            PanelFocus::Right => &mut self.right_list_state,
            PanelFocus::Other => &mut self.other_list_state,
            PanelFocus::Chords => &mut self.chord_list_state,
        }
    }

    fn move_selection_up(&mut self) {
        self.active_panel().select_previous();
    }

    fn move_selection_down(&mut self) {
        self.active_panel().select_next();
    }

    fn move_action_set_left(&mut self) {
        self.selected_action_set_index = self.selected_action_set_index.saturating_sub(1);
        self.clamp_other_selection();
    }

    fn move_action_set_right(&mut self) {
        if self.selected_action_set_index + 1 < self.action_sets.len() {
            self.selected_action_set_index += 1;
            self.clamp_other_selection();
        }
    }

    fn move_to_first(&mut self) {
        self.active_panel().select_first();
    }

    fn move_to_last(&mut self) {
        self.active_panel().select_last();
    }

    fn switch_panel(&mut self) {
        self.panel_focus = match (self.has_two_panels(), self.panel_focus) {
            (true, PanelFocus::Left) => PanelFocus::Right,
            (true, PanelFocus::Right) => PanelFocus::Other,
            (true, PanelFocus::Other) => PanelFocus::Chords,
            (true, PanelFocus::Chords) => PanelFocus::Left,
            (false, PanelFocus::Left | PanelFocus::Right) => PanelFocus::Other,
            (false, PanelFocus::Other) => PanelFocus::Chords,
            (false, PanelFocus::Chords) => PanelFocus::Left,
        };
    }

    fn switch_action_set_by_key(&mut self, key: KeyCode) {
        let KeyCode::Char(character) = key else {
            return;
        };
        let Some(index) = character.to_digit(10).map(|digit| digit as usize - 1) else {
            return;
        };

        if index < self.action_sets.len() {
            self.selected_action_set_index = index;
            self.clamp_other_selection();
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(7),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_title(frame, chunks[0]);
        self.render_action_set_tabs(frame, chunks[1]);
        self.render_source_panels(frame, chunks[2]);

        let bottom_panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);

        self.render_other_bindings(frame, bottom_panels[0]);
        self.render_chords_bindings(frame, bottom_panels[1]);

        let bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(chunks[4]);

        self.render_status(frame, bottom[0]);
        self.render_help(frame, bottom[1]);

        if let Some(modal) = self.binding_modal.as_mut() {
            render_source_binding_modal(frame, area, modal, &self.actions);
        }
        if let Some(modal) = self.other_modal.as_mut() {
            render_other_binding_modal(
                frame,
                area,
                modal,
                &self.actions,
                &self.path_sources,
                &self.chord_sources,
            );
        }
        if let Some(dialog) = self.save_path_dialog.as_ref() {
            render_save_path_dialog(frame, area, dialog);
        }
        if self.discard_dialog {
            render_discard_dialog(frame, area);
        }
    }

    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![Span::styled(
            format!(
                "{} bindings for {}{}",
                self.controller_display_name,
                self.game_name,
                if self.dirty { " *" } else { "" }
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center);

        frame.render_widget(title, area);
    }

    fn render_action_set_tabs(&self, frame: &mut Frame, area: Rect) {
        if self.action_sets.is_empty() {
            return;
        }

        let titles: Vec<Line> = self
            .action_sets
            .iter()
            .map(|set| {
                let display_name = match set.usage.as_ref() {
                    Some(ActionSetUsage::LeftRight) => {
                        format!("{} (L/R)", short_path(&set.name))
                    }
                    _ => short_path(&set.name).to_owned(),
                };
                Line::from(display_name)
            })
            .collect();

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .title("Action Sets")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .select(self.selected_action_set_index)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(Span::raw(" | "));

        frame.render_widget(tabs, area);
    }

    fn render_source_panels(&mut self, frame: &mut Frame, area: Rect) {
        if self.loading {
            frame.render_widget(
                Paragraph::new("Loading bindings...").alignment(Alignment::Center),
                area,
            );
            return;
        }

        if let Some(error) = &self.error {
            frame.render_widget(
                Paragraph::new(format!("Error: {error}"))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }

        let action_set_bindings = self
            .current_action_set_bindings()
            .cloned()
            .unwrap_or_default();

        if self.has_two_panels() {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            Self::render_source_panel(
                frame,
                panels[0],
                "Left Controller",
                &self.left_sources,
                &mut self.left_list_state,
                self.panel_focus == PanelFocus::Left,
                ControllerSide::Left,
                &action_set_bindings,
                &self.actions,
            );
            Self::render_source_panel(
                frame,
                panels[1],
                "Right Controller",
                &self.right_sources,
                &mut self.right_list_state,
                self.panel_focus == PanelFocus::Right,
                ControllerSide::Right,
                &action_set_bindings,
                &self.actions,
            );
        } else {
            let title = match &self.ui_mode {
                BindingUiMode::Hmd => "HMD",
                BindingUiMode::SingleDevice | BindingUiMode::Other(_) => "Controller",
                BindingUiMode::ControllerHanded => unreachable!(),
            };

            Self::render_source_panel(
                frame,
                area,
                title,
                &self.left_sources,
                &mut self.left_list_state,
                self.panel_focus == PanelFocus::Left,
                ControllerSide::Single,
                &action_set_bindings,
                &self.actions,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_source_panel(
        frame: &mut Frame,
        area: Rect,
        title: &str,
        sources: &[SourceInfo],
        list_state: &mut ListState,
        is_focused: bool,
        side: ControllerSide,
        action_set_bindings: &ActionSetBindings,
        actions: &BTreeMap<String, ActionType>,
    ) {
        let border_color = if is_focused {
            Color::Yellow
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let items: Vec<ListItem<'static>> = sources
            .iter()
            .map(|source| source_list_item(source, side, action_set_bindings.sources(), actions))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_symbol("▶ ")
            .highlight_style(Style::default().bg(Color::DarkGray));

        if is_focused {
            frame.render_stateful_widget(list, area, list_state);
        } else {
            // Keep the panel's selection and scroll position internally, but do not
            // render a selected row while another panel has focus.
            let mut inactive_state = list_state.clone();
            inactive_state.select(None);
            frame.render_stateful_widget(list, area, &mut inactive_state);
        }
    }

    fn render_other_bindings(&mut self, frame: &mut Frame, area: Rect) {
        if self.loading || self.error.is_some() {
            return;
        }

        let bindings = self
            .current_action_set_bindings()
            .cloned()
            .unwrap_or_default();
        let entries = self.other_path_entries();
        let items = entries
            .iter()
            .map(|entry| {
                other_binding_list_item(
                    entry,
                    &bindings,
                    &self.actions,
                    &self.path_sources,
                    &self.chord_sources,
                )
            })
            .collect::<Vec<_>>();

        let is_focused = self.panel_focus == PanelFocus::Other;
        let border_color = if is_focused {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let block = Block::default()
            .title("Path Bindings")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let list = List::new(items)
            .block(block)
            .highlight_symbol("▶ ")
            .highlight_style(Style::default().bg(Color::DarkGray));

        if is_focused {
            frame.render_stateful_widget(list, area, &mut self.other_list_state);
        } else {
            let mut inactive_state = self.other_list_state.clone();
            inactive_state.select(None);
            frame.render_stateful_widget(list, area, &mut inactive_state);
        }
    }

    fn render_chords_bindings(&mut self, frame: &mut Frame, area: Rect) {
        if self.loading || self.error.is_some() {
            return;
        }

        let bindings = self
            .current_action_set_bindings()
            .cloned()
            .unwrap_or_default();
        let entries = self.chord_binding_entries();
        let items = entries
            .iter()
            .map(|entry| {
                other_binding_list_item(
                    entry,
                    &bindings,
                    &self.actions,
                    &self.path_sources,
                    &self.chord_sources,
                )
            })
            .collect::<Vec<_>>();

        let is_focused = self.panel_focus == PanelFocus::Chords;
        let border_color = if is_focused {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let block = Block::default()
            .title("Chords")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let list = List::new(items)
            .block(block)
            .highlight_symbol("▶ ")
            .highlight_style(Style::default().bg(Color::DarkGray));

        if is_focused {
            frame.render_stateful_widget(list, area, &mut self.chord_list_state);
        } else {
            let mut inactive_state = self.chord_list_state.clone();
            inactive_state.select(None);
            frame.render_stateful_widget(list, area, &mut inactive_state);
        }
    }

    fn current_action_set_bindings(&self) -> Option<&ActionSetBindings> {
        let action_set = self.action_sets.get(self.selected_action_set_index)?;
        find_action_set_bindings(&self.binding.bindings, &action_set.name)
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        if let Some(message) = &self.status_message {
            frame.render_widget(
                Paragraph::new(Span::styled(message, Style::default().fg(Color::Yellow)))
                    .alignment(Alignment::Center),
                area,
            );
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = Paragraph::new(
            "↑/↓ select • Enter edit/add • ←/→ action set • Tab switch panel • 1-9 jump set • w save • q quit",
        )
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

        frame.render_widget(help, area);
    }
}

impl SourceBindingModal {
    fn new(
        action_set_name: String,
        source: SourceInfo,
        side: ControllerSide,
        new_binding_path: String,
        drafts: Vec<SourceBinding>,
    ) -> Self {
        Self {
            action_set_name,
            source,
            side,
            new_binding_path,
            drafts,
            selected_binding: 0,
            selected_field: 0,
            focus: BindingModalFocus::BindingList,
            mode_picker: None,
            action_picker: None,
            parameter_picker: None,
            parameter_editor: None,
        }
    }

    fn handle_key(&mut self, key: KeyCode, actions: &BTreeMap<String, ActionType>) -> ModalOutcome {
        if self.parameter_editor.is_some() {
            self.handle_parameter_editor_key(key);
            return ModalOutcome::Continue;
        }

        if self.parameter_picker.is_some() {
            self.handle_parameter_picker_key(key);
            return ModalOutcome::Continue;
        }

        if self.mode_picker.is_some() {
            self.handle_mode_picker_key(key);
            return ModalOutcome::Continue;
        }

        if self.action_picker.is_some() {
            self.handle_action_picker_key(key);
            return ModalOutcome::Continue;
        }

        if key == KeyCode::Char('s') {
            return ModalOutcome::Apply;
        }

        if key == KeyCode::Char('n') {
            self.add_binding();
            return ModalOutcome::Continue;
        }

        match self.focus {
            BindingModalFocus::BindingList => self.handle_binding_list_key(key),
            BindingModalFocus::Editor => self.handle_editor_key(key, actions),
        }
    }

    fn handle_binding_list_key(&mut self, key: KeyCode) -> ModalOutcome {
        match key {
            KeyCode::Esc => return ModalOutcome::Cancel,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_binding = self.selected_binding.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_binding =
                    (self.selected_binding + 1).min(self.binding_list_len() - 1);
            }
            KeyCode::Home | KeyCode::PageUp => self.selected_binding = 0,
            KeyCode::End | KeyCode::PageDown => {
                self.selected_binding = self.binding_list_len() - 1;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.selected_binding == self.drafts.len() {
                    self.add_binding();
                } else {
                    self.focus = BindingModalFocus::Editor;
                    self.selected_field = 0;
                }
            }
            KeyCode::Tab => {
                if self.selected_binding < self.drafts.len() {
                    self.focus = BindingModalFocus::Editor;
                    self.selected_field = 0;
                }
            }
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('d') => {
                self.remove_selected_binding();
            }
            _ => {}
        }

        ModalOutcome::Continue
    }

    fn handle_editor_key(
        &mut self,
        key: KeyCode,
        actions: &BTreeMap<String, ActionType>,
    ) -> ModalOutcome {
        if self.selected_binding >= self.drafts.len() {
            self.focus = BindingModalFocus::BindingList;
            self.selected_binding = self.drafts.len();
            return ModalOutcome::Continue;
        }

        match key {
            KeyCode::Esc | KeyCode::Tab => {
                self.focus = BindingModalFocus::BindingList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_field = self.selected_field.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_field = (self.selected_field + 1).min(self.editor_field_count() - 1);
            }
            KeyCode::Home | KeyCode::PageUp => self.selected_field = 0,
            KeyCode::End | KeyCode::PageDown => {
                self.selected_field = self.editor_field_count() - 1;
            }
            KeyCode::Enter | KeyCode::Char(' ') => match self.selected_editor_field() {
                Some(SourceEditorField::Mode) => self.open_mode_picker(),
                Some(SourceEditorField::Input(_)) => self.open_action_picker(actions),
                Some(SourceEditorField::Parameter(name)) => self.open_parameter_editor(&name),
                Some(SourceEditorField::AddParameter) => self.open_parameter_picker(),
                None => {}
            },
            KeyCode::Left | KeyCode::Char('h') => match self.selected_editor_field() {
                Some(SourceEditorField::Mode) => self.cycle_mode(false),
                Some(SourceEditorField::Input(_)) => self.cycle_action(false, actions),
                Some(SourceEditorField::Parameter(name)) => {
                    self.cycle_parameter_value(&name, false)
                }
                Some(SourceEditorField::AddParameter) | None => {}
            },
            KeyCode::Right | KeyCode::Char('l') => match self.selected_editor_field() {
                Some(SourceEditorField::Mode) => self.cycle_mode(true),
                Some(SourceEditorField::Input(_)) => self.cycle_action(true, actions),
                Some(SourceEditorField::Parameter(name)) => self.cycle_parameter_value(&name, true),
                Some(SourceEditorField::AddParameter) | None => {}
            },
            KeyCode::Delete | KeyCode::Backspace => match self.selected_editor_field() {
                Some(SourceEditorField::Input(input)) => {
                    if let Some(binding) = self.drafts.get_mut(self.selected_binding) {
                        binding.inputs.remove(&input);
                    }
                }
                Some(SourceEditorField::Parameter(name)) => self.remove_parameter(&name),
                _ => {}
            },
            KeyCode::Char('p') => self.open_parameter_picker(),
            KeyCode::Char('d') => self.remove_selected_binding(),
            _ => {}
        }

        ModalOutcome::Continue
    }

    fn handle_mode_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.mode_picker.take() else {
            return;
        };

        match key {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
                picker.selected = picker.selected.saturating_sub(1);
                self.mode_picker = Some(picker);
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
                self.mode_picker = Some(picker);
            }
            KeyCode::Home | KeyCode::PageUp => {
                picker.selected = 0;
                self.mode_picker = Some(picker);
            }
            KeyCode::End | KeyCode::PageDown => {
                picker.selected = picker.options.len() - 1;
                self.mode_picker = Some(picker);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let mode = picker.options[picker.selected].clone();
                self.set_selected_mode(mode);
            }
            _ => self.mode_picker = Some(picker),
        }
    }

    fn handle_action_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.action_picker.take() else {
            return;
        };

        match key {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::Char('k') => {
                picker.selected = picker.selected.saturating_sub(1);
                self.action_picker = Some(picker);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
                self.action_picker = Some(picker);
            }
            KeyCode::Home | KeyCode::PageUp => {
                picker.selected = 0;
                self.action_picker = Some(picker);
            }
            KeyCode::End | KeyCode::PageDown => {
                picker.selected = picker.options.len() - 1;
                self.action_picker = Some(picker);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let input = picker.input;
                let output = picker.options[picker.selected].clone();
                self.set_selected_action(input, output);
            }
            _ => self.action_picker = Some(picker),
        }
    }

    fn binding_list_len(&self) -> usize {
        self.drafts.len() + 1
    }

    fn editor_fields(&self) -> Vec<SourceEditorField> {
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return vec![SourceEditorField::Mode];
        };

        let mut fields = vec![SourceEditorField::Mode];
        fields.extend(
            display_inputs(&self.source.def, binding)
                .into_iter()
                .map(SourceEditorField::Input),
        );
        if let Some(parameters) = binding.parameters.as_ref() {
            fields.extend(parameters.keys().cloned().map(SourceEditorField::Parameter));
        }
        fields.push(SourceEditorField::AddParameter);
        fields
    }

    fn editor_field_count(&self) -> usize {
        self.editor_fields().len().max(1)
    }

    fn selected_editor_field(&self) -> Option<SourceEditorField> {
        self.editor_fields().get(self.selected_field).cloned()
    }

    fn selected_input(&self) -> Option<BindingInput> {
        match self.selected_editor_field()? {
            SourceEditorField::Input(input) => Some(input),
            _ => None,
        }
    }

    fn add_binding(&mut self) {
        let Some(mode) = supported_modes(&self.source.def).into_iter().next() else {
            return;
        };

        self.drafts.push(SourceBinding {
            path: self.new_binding_path.clone(),
            mode,
            inputs: BTreeMap::new(),
            parameters: None,
            extra: BTreeMap::new(),
        });
        self.selected_binding = self.drafts.len() - 1;
        self.selected_field = 0;
        self.focus = BindingModalFocus::Editor;
    }

    fn remove_selected_binding(&mut self) {
        if self.selected_binding >= self.drafts.len() {
            return;
        }

        self.drafts.remove(self.selected_binding);
        self.selected_binding = self.selected_binding.min(self.drafts.len());
        self.selected_field = 0;
        self.focus = BindingModalFocus::BindingList;
    }

    fn open_mode_picker(&mut self) {
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return;
        };

        let mut options = supported_modes(&self.source.def);
        if !options.contains(&binding.mode) {
            options.push(binding.mode.clone());
        }
        if options.is_empty() {
            return;
        }

        let selected = options
            .iter()
            .position(|mode| mode == &binding.mode)
            .unwrap_or(0);
        self.mode_picker = Some(ModePicker { options, selected });
    }

    fn open_action_picker(&mut self, actions: &BTreeMap<String, ActionType>) {
        let Some(input) = self.selected_input() else {
            return;
        };
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return;
        };

        let current = binding
            .inputs
            .get(&input)
            .map(|output| output.output.as_str());
        let options = compatible_action_options(
            actions,
            &self.action_set_name,
            &binding.mode,
            &input,
            current,
        );
        let selected = current
            .and_then(|current| {
                options.iter().position(|option| {
                    option
                        .as_deref()
                        .is_some_and(|option| option.eq_ignore_ascii_case(current))
                })
            })
            .unwrap_or(0);

        self.action_picker = Some(ActionPicker {
            input,
            options,
            selected,
        });
    }

    fn cycle_mode(&mut self, forward: bool) {
        let Some(current) = self
            .drafts
            .get(self.selected_binding)
            .map(|binding| binding.mode.clone())
        else {
            return;
        };

        let mut modes = supported_modes(&self.source.def);
        if !modes.contains(&current) {
            modes.push(current.clone());
        }
        if modes.is_empty() {
            return;
        }

        let index = modes.iter().position(|mode| mode == &current).unwrap_or(0);
        let next = step_index(index, modes.len(), forward);
        self.set_selected_mode(modes[next].clone());
    }

    fn cycle_action(&mut self, forward: bool, actions: &BTreeMap<String, ActionType>) {
        let Some(input) = self.selected_input() else {
            return;
        };
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return;
        };

        let current = binding
            .inputs
            .get(&input)
            .map(|output| output.output.as_str());
        let options = compatible_action_options(
            actions,
            &self.action_set_name,
            &binding.mode,
            &input,
            current,
        );
        let index = current
            .and_then(|current| {
                options.iter().position(|option| {
                    option
                        .as_deref()
                        .is_some_and(|option| option.eq_ignore_ascii_case(current))
                })
            })
            .unwrap_or(0);
        let next = step_index(index, options.len(), forward);
        self.set_selected_action(input, options[next].clone());
    }

    fn open_parameter_picker(&mut self) {
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return;
        };

        let mut options = known_parameter_specs(&self.source.def, &binding.mode)
            .into_iter()
            .filter(|spec| {
                !binding.parameters.as_ref().is_some_and(|parameters| {
                    parameters
                        .keys()
                        .any(|name| name.eq_ignore_ascii_case(spec.name))
                })
            })
            .map(|spec| ParameterChoice {
                name: Some(spec.name.to_owned()),
                label: format!("{} ({})", spec.label, spec.name),
                default_value: spec.default_value,
            })
            .collect::<Vec<_>>();

        options.push(ParameterChoice {
            name: None,
            label: "Custom / unknown parameter".to_owned(),
            default_value: String::new(),
        });
        self.parameter_picker = Some(ParameterPicker {
            options,
            selected: 0,
        });
    }

    fn handle_parameter_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.parameter_picker.take() else {
            return;
        };

        match key {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
                picker.selected = picker.selected.saturating_sub(1);
                self.parameter_picker = Some(picker);
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
                self.parameter_picker = Some(picker);
            }
            KeyCode::Home | KeyCode::PageUp => {
                picker.selected = 0;
                self.parameter_picker = Some(picker);
            }
            KeyCode::End | KeyCode::PageDown => {
                picker.selected = picker.options.len() - 1;
                self.parameter_picker = Some(picker);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let choice = picker.options[picker.selected].clone();
                let known = choice.name.is_some();
                self.parameter_editor = Some(ParameterEditor {
                    original_name: None,
                    name: choice.name.unwrap_or_default(),
                    value: choice.default_value,
                    selected_field: if known { 1 } else { 0 },
                    error: None,
                });
            }
            _ => self.parameter_picker = Some(picker),
        }
    }

    fn open_parameter_editor(&mut self, name: &str) {
        let Some(value) = self
            .drafts
            .get(self.selected_binding)
            .and_then(|binding| binding.parameters.as_ref())
            .and_then(|parameters| parameters.get(name))
        else {
            return;
        };

        self.parameter_editor = Some(ParameterEditor {
            original_name: Some(name.to_owned()),
            name: name.to_owned(),
            value: parameter_value_edit_text(value),
            selected_field: 1,
            error: None,
        });
    }

    fn handle_parameter_editor_key(&mut self, key: KeyCode) {
        let Some(mut editor) = self.parameter_editor.take() else {
            return;
        };

        match key {
            KeyCode::Esc => return,
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                editor.selected_field = 1 - editor.selected_field;
            }
            KeyCode::Enter => match self.apply_parameter_editor(&editor) {
                Ok(()) => return,
                Err(error) => editor.error = Some(error),
            },
            KeyCode::Backspace => {
                let field = if editor.selected_field == 0 {
                    &mut editor.name
                } else {
                    &mut editor.value
                };
                field.pop();
                editor.error = None;
            }
            KeyCode::Delete => {
                if editor.selected_field == 0 {
                    editor.name.clear();
                } else {
                    editor.value.clear();
                }
                editor.error = None;
            }
            KeyCode::Char(character) => {
                let field = if editor.selected_field == 0 {
                    &mut editor.name
                } else {
                    &mut editor.value
                };
                field.push(character);
                editor.error = None;
            }
            _ => {}
        }

        self.parameter_editor = Some(editor);
    }

    fn apply_parameter_editor(&mut self, editor: &ParameterEditor) -> Result<(), String> {
        let name = editor.name.trim();
        if name.is_empty() {
            return Err("Parameter name cannot be empty".to_owned());
        }

        let value = parse_parameter_value(&editor.value)?;
        let Some(binding) = self.drafts.get_mut(self.selected_binding) else {
            return Err("The selected binding no longer exists".to_owned());
        };
        let parameters = binding.parameters.get_or_insert_with(BTreeMap::new);

        let conflicts_with_existing = parameters.keys().any(|existing| {
            existing.eq_ignore_ascii_case(name)
                && !editor
                    .original_name
                    .as_deref()
                    .is_some_and(|original| existing.eq_ignore_ascii_case(original))
        });
        if conflicts_with_existing {
            return Err(format!("A parameter named {name} already exists"));
        }

        if let Some(original) = editor.original_name.as_deref() {
            if original != name {
                parameters.remove(original);
            }
        }
        parameters.insert(name.to_owned(), value);
        Ok(())
    }

    fn remove_parameter(&mut self, name: &str) {
        let Some(binding) = self.drafts.get_mut(self.selected_binding) else {
            return;
        };
        if let Some(parameters) = binding.parameters.as_mut() {
            parameters.remove(name);
            if parameters.is_empty() {
                binding.parameters = None;
            }
        }
        self.selected_field = self.selected_field.min(self.editor_field_count() - 1);
    }

    fn cycle_parameter_value(&mut self, name: &str, forward: bool) {
        let Some(binding) = self.drafts.get(self.selected_binding) else {
            return;
        };
        let Some(spec) = known_parameter_specs(&self.source.def, &binding.mode)
            .into_iter()
            .find(|spec| spec.name.eq_ignore_ascii_case(name))
        else {
            return;
        };
        if spec.choices.is_empty() {
            return;
        }

        let current = binding
            .parameters
            .as_ref()
            .and_then(|parameters| parameters.get(name))
            .map(compact_json_value)
            .unwrap_or_else(|| spec.default_value.clone());
        let index = spec
            .choices
            .iter()
            .position(|choice| choice.eq_ignore_ascii_case(&current))
            .unwrap_or(0);
        let next = step_index(index, spec.choices.len(), forward);
        if let Some(binding) = self.drafts.get_mut(self.selected_binding) {
            binding.parameters.get_or_insert_with(BTreeMap::new).insert(
                name.to_owned(),
                serde_json::Value::String(spec.choices[next].clone()),
            );
        }
    }

    fn set_selected_mode(&mut self, mode: BindingMode) {
        let allowed_inputs = expected_inputs(&self.source.def, &mode);
        let Some(binding) = self.drafts.get_mut(self.selected_binding) else {
            return;
        };

        if binding.mode != mode {
            binding.mode = mode;
            binding
                .inputs
                .retain(|input, _| allowed_inputs.contains(input));
        }

        self.selected_field = self.selected_field.min(self.editor_field_count() - 1);
    }

    fn set_selected_action(&mut self, input: BindingInput, output: Option<String>) {
        let Some(binding) = self.drafts.get_mut(self.selected_binding) else {
            return;
        };

        if let Some(output) = output {
            binding.inputs.insert(
                input,
                BindingOutput {
                    output,
                    extra: BTreeMap::new(),
                },
            );
        } else {
            binding.inputs.remove(&input);
        }
    }
}

impl PathBindingModal {
    fn new(
        action_set_name: String,
        kind: PathBindingKind,
        original_index: Option<usize>,
        draft: PathBinding,
    ) -> Self {
        Self {
            action_set_name,
            kind,
            original_index,
            draft,
            selected_field: 0,
            source_picker: None,
            action_picker: None,
            error: None,
        }
    }

    fn handle_key(
        &mut self,
        key: KeyCode,
        actions: &BTreeMap<String, ActionType>,
        sources: &[PathSourceInfo],
    ) -> OtherModalOutcome {
        if self.source_picker.is_some() {
            self.handle_source_picker_key(key);
            return OtherModalOutcome::Continue;
        }
        if self.action_picker.is_some() {
            self.handle_action_picker_key(key);
            return OtherModalOutcome::Continue;
        }

        match key {
            KeyCode::Esc => return OtherModalOutcome::Cancel,
            KeyCode::Char('s') => {
                if self.draft.path.is_empty() {
                    self.error = Some("Select a physical source".to_owned());
                } else if self.draft.output.is_empty() {
                    self.error = Some("Select an application action".to_owned());
                } else {
                    return OtherModalOutcome::Apply;
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                return if self.original_index.is_some() {
                    OtherModalOutcome::Delete
                } else {
                    OtherModalOutcome::Cancel
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_field = self.selected_field.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_field = (self.selected_field + 1).min(1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.selected_field == 0 {
                    self.open_source_picker(sources);
                } else {
                    self.open_action_picker(actions);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_field == 0 {
                    self.cycle_source(false, sources);
                } else {
                    self.cycle_action(false, actions);
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected_field == 0 {
                    self.cycle_source(true, sources);
                } else {
                    self.cycle_action(true, actions);
                }
            }
            _ => {}
        }

        OtherModalOutcome::Continue
    }

    fn source_choices(&self, sources: &[PathSourceInfo]) -> Vec<StringChoice> {
        let mut choices = sources
            .iter()
            .filter(|source| source.kind == self.kind)
            .map(|source| StringChoice {
                value: source.path.clone(),
                label: source.name.clone(),
            })
            .collect::<Vec<_>>();
        add_current_choice(&mut choices, &self.draft.path, short_path(&self.draft.path));
        choices
    }

    fn action_choices(&self, actions: &BTreeMap<String, ActionType>) -> Vec<StringChoice> {
        action_choices(
            actions,
            &self.action_set_name,
            &self.kind.action_type(),
            self.kind.action_direction(),
            Some(&self.draft.output),
        )
    }

    fn open_source_picker(&mut self, sources: &[PathSourceInfo]) {
        let options = self.source_choices(sources);
        if options.is_empty() {
            return;
        }
        let selected = choice_index(&options, &self.draft.path);
        self.source_picker = Some(StringPicker {
            title: format!("Select {} source", self.kind.display_name()),
            options,
            selected,
        });
    }

    fn open_action_picker(&mut self, actions: &BTreeMap<String, ActionType>) {
        let options = self.action_choices(actions);
        if options.is_empty() {
            return;
        }
        let selected = choice_index(&options, &self.draft.output);
        self.action_picker = Some(StringPicker {
            title: format!("Select {} action", self.kind.display_name()),
            options,
            selected,
        });
    }

    fn handle_source_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.source_picker.take() else {
            return;
        };
        match handle_string_picker_key(key, &mut picker) {
            PickerOutcome::Continue => self.source_picker = Some(picker),
            PickerOutcome::Cancel => {}
            PickerOutcome::Apply => {
                self.draft.path = picker.options[picker.selected].value.clone();
                self.error = None;
            }
        }
    }

    fn handle_action_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.action_picker.take() else {
            return;
        };
        match handle_string_picker_key(key, &mut picker) {
            PickerOutcome::Continue => self.action_picker = Some(picker),
            PickerOutcome::Cancel => {}
            PickerOutcome::Apply => {
                self.draft.output = picker.options[picker.selected].value.clone();
                self.error = None;
            }
        }
    }

    fn cycle_source(&mut self, forward: bool, sources: &[PathSourceInfo]) {
        let options = self.source_choices(sources);
        cycle_choice_value(&mut self.draft.path, &options, forward);
        self.error = None;
    }

    fn cycle_action(&mut self, forward: bool, actions: &BTreeMap<String, ActionType>) {
        let options = self.action_choices(actions);
        cycle_choice_value(&mut self.draft.output, &options, forward);
        self.error = None;
    }
}

impl ChordBindingModal {
    fn new(action_set_name: String, original_index: Option<usize>, draft: ChordBinding) -> Self {
        Self {
            action_set_name,
            original_index,
            draft,
            selected_field: 0,
            action_picker: None,
            source_picker: None,
            kind_picker: None,
            error: None,
        }
    }

    fn handle_key(
        &mut self,
        key: KeyCode,
        actions: &BTreeMap<String, ActionType>,
        sources: &[ChordSourceInfo],
    ) -> OtherModalOutcome {
        if self.action_picker.is_some() {
            self.handle_action_picker_key(key);
            return OtherModalOutcome::Continue;
        }
        if self.source_picker.is_some() {
            self.handle_source_picker_key(key);
            return OtherModalOutcome::Continue;
        }
        if self.kind_picker.is_some() {
            self.handle_kind_picker_key(key);
            return OtherModalOutcome::Continue;
        }

        match key {
            KeyCode::Esc => return OtherModalOutcome::Cancel,
            KeyCode::Char('s') => {
                if self.draft.output.is_empty() {
                    self.error = Some("Select an application action".to_owned());
                } else if self.draft.inputs.is_empty() {
                    self.error = Some("A chord needs at least one input".to_owned());
                } else if self.draft.inputs.iter().any(|input| input.0.is_empty()) {
                    self.error = Some("Every chord input needs a physical source".to_owned());
                } else {
                    return OtherModalOutcome::Apply;
                }
            }
            KeyCode::Char('d') => {
                return if self.original_index.is_some() {
                    OtherModalOutcome::Delete
                } else {
                    OtherModalOutcome::Cancel
                };
            }
            KeyCode::Char('n') => self.add_input(sources),
            KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('x') => {
                self.remove_selected_input();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_field = self.selected_field.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_field =
                    (self.selected_field + 1).min(self.field_count().saturating_sub(1));
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.activate_selected_field(actions, sources),
            KeyCode::Left | KeyCode::Char('h') => {
                self.cycle_selected_field(false, actions, sources)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.cycle_selected_field(true, actions, sources)
            }
            _ => {}
        }

        OtherModalOutcome::Continue
    }

    fn field_count(&self) -> usize {
        2 + self.draft.inputs.len() * 2
    }

    fn add_field_index(&self) -> usize {
        1 + self.draft.inputs.len() * 2
    }

    fn input_field(&self) -> Option<(usize, bool)> {
        if self.selected_field == 0 || self.selected_field == self.add_field_index() {
            return None;
        }
        let relative = self.selected_field - 1;
        Some((relative / 2, relative % 2 == 0))
    }

    fn activate_selected_field(
        &mut self,
        actions: &BTreeMap<String, ActionType>,
        sources: &[ChordSourceInfo],
    ) {
        if self.selected_field == 0 {
            self.open_action_picker(actions);
        } else if self.selected_field == self.add_field_index() {
            self.add_input(sources);
        } else if let Some((index, is_source)) = self.input_field() {
            if is_source {
                self.open_source_picker(index, sources);
            } else {
                self.open_kind_picker(index);
            }
        }
    }

    fn cycle_selected_field(
        &mut self,
        forward: bool,
        actions: &BTreeMap<String, ActionType>,
        sources: &[ChordSourceInfo],
    ) {
        if self.selected_field == 0 {
            let options = action_choices(
                actions,
                &self.action_set_name,
                &ActionType::Boolean,
                ActionDirection::Input,
                Some(&self.draft.output),
            );
            cycle_choice_value(&mut self.draft.output, &options, forward);
        } else if let Some((index, is_source)) = self.input_field() {
            if is_source {
                let current = self.draft.inputs[index].0.clone();
                let options = chord_source_choices(sources, Some(&current));
                cycle_choice_value(&mut self.draft.inputs[index].0, &options, forward);
            } else {
                let options = chord_kind_options(&self.draft.inputs[index].1);
                let current = self.draft.inputs[index].1.clone();
                let current_index = options
                    .iter()
                    .position(|kind| kind == &current)
                    .unwrap_or(0);
                self.draft.inputs[index].1 =
                    options[step_index(current_index, options.len(), forward)].clone();
            }
        }
        self.error = None;
    }

    fn add_input(&mut self, sources: &[ChordSourceInfo]) {
        let path = sources
            .first()
            .map(|source| source.path.clone())
            .unwrap_or_default();
        self.draft
            .inputs
            .push(ChordInput(path, ChordInputKind::Held));
        self.selected_field = 1 + (self.draft.inputs.len() - 1) * 2;
        self.error = None;
    }

    fn remove_selected_input(&mut self) {
        let Some((index, _)) = self.input_field() else {
            return;
        };
        if index < self.draft.inputs.len() {
            self.draft.inputs.remove(index);
            self.selected_field = self.selected_field.min(self.add_field_index());
            self.error = None;
        }
    }

    fn open_action_picker(&mut self, actions: &BTreeMap<String, ActionType>) {
        let options = action_choices(
            actions,
            &self.action_set_name,
            &ActionType::Boolean,
            ActionDirection::Input,
            Some(&self.draft.output),
        );
        if options.is_empty() {
            return;
        }
        let selected = choice_index(&options, &self.draft.output);
        self.action_picker = Some(StringPicker {
            title: "Select chord action".to_owned(),
            options,
            selected,
        });
    }

    fn open_source_picker(&mut self, input_index: usize, sources: &[ChordSourceInfo]) {
        let Some(input) = self.draft.inputs.get(input_index) else {
            return;
        };
        let options = chord_source_choices(sources, Some(&input.0));
        if options.is_empty() {
            return;
        }
        let selected = choice_index(&options, &input.0);
        self.source_picker = Some((
            input_index,
            StringPicker {
                title: format!("Select chord input #{}", input_index + 1),
                options,
                selected,
            },
        ));
    }

    fn open_kind_picker(&mut self, input_index: usize) {
        let Some(input) = self.draft.inputs.get(input_index) else {
            return;
        };
        let options = chord_kind_options(&input.1);
        let selected = options
            .iter()
            .position(|kind| kind == &input.1)
            .unwrap_or(0);
        self.kind_picker = Some(ChordKindPicker {
            input_index,
            options,
            selected,
        });
    }

    fn handle_action_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.action_picker.take() else {
            return;
        };
        match handle_string_picker_key(key, &mut picker) {
            PickerOutcome::Continue => self.action_picker = Some(picker),
            PickerOutcome::Cancel => {}
            PickerOutcome::Apply => {
                self.draft.output = picker.options[picker.selected].value.clone();
                self.error = None;
            }
        }
    }

    fn handle_source_picker_key(&mut self, key: KeyCode) {
        let Some((input_index, mut picker)) = self.source_picker.take() else {
            return;
        };
        match handle_string_picker_key(key, &mut picker) {
            PickerOutcome::Continue => self.source_picker = Some((input_index, picker)),
            PickerOutcome::Cancel => {}
            PickerOutcome::Apply => {
                if let Some(input) = self.draft.inputs.get_mut(input_index) {
                    input.0 = picker.options[picker.selected].value.clone();
                    self.error = None;
                }
            }
        }
    }

    fn handle_kind_picker_key(&mut self, key: KeyCode) {
        let Some(mut picker) = self.kind_picker.take() else {
            return;
        };
        match key {
            KeyCode::Esc => {}
            KeyCode::Up | KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
                picker.selected = picker.selected.saturating_sub(1);
                self.kind_picker = Some(picker);
            }
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
                self.kind_picker = Some(picker);
            }
            KeyCode::Home | KeyCode::PageUp => {
                picker.selected = 0;
                self.kind_picker = Some(picker);
            }
            KeyCode::End | KeyCode::PageDown => {
                picker.selected = picker.options.len() - 1;
                self.kind_picker = Some(picker);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(input) = self.draft.inputs.get_mut(picker.input_index) {
                    input.1 = picker.options[picker.selected].clone();
                    self.error = None;
                }
            }
            _ => self.kind_picker = Some(picker),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerOutcome {
    Continue,
    Apply,
    Cancel,
}

fn handle_string_picker_key(key: KeyCode, picker: &mut StringPicker) -> PickerOutcome {
    match key {
        KeyCode::Esc => PickerOutcome::Cancel,
        KeyCode::Up | KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
            picker.selected = picker.selected.saturating_sub(1);
            PickerOutcome::Continue
        }
        KeyCode::Down | KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
            picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
            PickerOutcome::Continue
        }
        KeyCode::Home | KeyCode::PageUp => {
            picker.selected = 0;
            PickerOutcome::Continue
        }
        KeyCode::End | KeyCode::PageDown => {
            picker.selected = picker.options.len() - 1;
            PickerOutcome::Continue
        }
        KeyCode::Enter | KeyCode::Char(' ') => PickerOutcome::Apply,
        _ => PickerOutcome::Continue,
    }
}

fn render_source_binding_modal(
    frame: &mut Frame,
    screen_area: Rect,
    modal: &mut SourceBindingModal,
    actions: &BTreeMap<String, ActionType>,
) {
    let area = centered_rect(90, 86, screen_area);
    frame.render_widget(Clear, area);

    let title = format!(
        " Edit {} bindings - {} ",
        modal.source.name,
        short_path(&modal.action_set_name)
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                modal.new_binding_path.clone(),
                Style::default().fg(Color::Cyan),
            ),
        ])),
        rows[0],
    );

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(rows[1]);

    render_modal_binding_list(frame, columns[0], modal, actions);
    render_modal_binding_editor(frame, columns[1], modal, actions);

    let help = if modal.parameter_editor.is_some() {
        "Type name/value • Tab switch field • Enter apply • Esc cancel"
    } else if modal.mode_picker.is_some()
        || modal.action_picker.is_some()
        || modal.parameter_picker.is_some()
    {
        "↑/↓ choose • Enter apply • Esc close picker"
    } else {
        match modal.focus {
            BindingModalFocus::BindingList => {
                "↑/↓ binding • Enter edit/add • n add • d/Del remove • s apply • Esc cancel"
            }
            BindingModalFocus::Editor => {
                "↑/↓ field • ←/→ cycle • Enter edit • p add parameter • Del clear • d remove • s apply"
            }
        }
    };
    frame.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center),
        rows[2],
    );

    if modal.parameter_editor.is_some() {
        render_parameter_editor(frame, area, modal);
    } else if modal.parameter_picker.is_some() {
        render_parameter_picker(frame, area, modal);
    } else if modal.mode_picker.is_some() {
        render_mode_picker(frame, area, modal);
    } else if modal.action_picker.is_some() {
        render_action_picker(frame, area, modal, actions);
    }
}

fn render_modal_binding_list(
    frame: &mut Frame,
    area: Rect,
    modal: &SourceBindingModal,
    actions: &BTreeMap<String, ActionType>,
) {
    let items = modal
        .drafts
        .iter()
        .enumerate()
        .map(|(index, binding)| {
            let mapped = binding
                .inputs
                .values()
                .map(|output| action_display_name(actions, &output.output))
                .collect::<Vec<_>>()
                .join(", ");
            let suffix = if mapped.is_empty() {
                "unbound".to_owned()
            } else {
                mapped
            };
            ListItem::new(format!(
                "#{}  {}  ({})",
                index + 1,
                binding.mode.as_str(),
                suffix
            ))
        })
        .chain(std::iter::once(ListItem::new("[+] Add binding")))
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    state.select(Some(modal.selected_binding));
    let border = if modal.focus == BindingModalFocus::BindingList {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let list = List::new(items)
        .block(
            Block::default()
                .title("Bindings")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_modal_binding_editor(
    frame: &mut Frame,
    area: Rect,
    modal: &SourceBindingModal,
    actions: &BTreeMap<String, ActionType>,
) {
    let border = if modal.focus == BindingModalFocus::Editor {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let Some(binding) = modal.drafts.get(modal.selected_binding) else {
        frame.render_widget(
            Paragraph::new("Select [+] Add binding and press Enter")
                .block(
                    Block::default()
                        .title("Binding Editor")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border)),
                )
                .alignment(Alignment::Center),
            area,
        );
        return;
    };

    let mut items = Vec::new();
    items.push(ListItem::new(Line::from(vec![
        Span::styled("Mode       ", Style::default().fg(Color::Gray)),
        Span::styled(
            binding.mode.as_str().to_owned(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ])));

    for input in display_inputs(&modal.source.def, binding) {
        let output = binding.inputs.get(&input);
        let (label, style) = match output {
            Some(output) => (
                action_display_name(actions, &output.output),
                action_output_style(&binding.mode, &input, &output.output, actions),
            ),
            None => ("None".to_owned(), Style::default().fg(Color::DarkGray)),
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<11}", input.as_str()),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(label, style),
        ])));
    }

    if let Some(parameters) = binding.parameters.as_ref() {
        let known = known_parameter_specs(&modal.source.def, &binding.mode);
        let name_width = parameters.keys().map(|name| name.len()).max().unwrap_or(0);

        for (name, value) in parameters {
            let style = if known
                .iter()
                .any(|spec| spec.name.eq_ignore_ascii_case(name))
            {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::Yellow)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("param {name:<name_width$}  "),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(compact_json_value(value), style),
            ])));
        }
    }
    items.push(ListItem::new(Line::from(Span::styled(
        "[+] Add parameter",
        Style::default().fg(Color::Blue),
    ))));

    let mut state = ListState::default();
    state.select(Some(modal.selected_field));
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Binding #{}", modal.selected_binding + 1))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_parameter_picker(frame: &mut Frame, parent: Rect, modal: &SourceBindingModal) {
    let Some(picker) = modal.parameter_picker.as_ref() else {
        return;
    };
    let area = centered_rect(64, 68, parent);
    frame.render_widget(Clear, area);
    let items = picker
        .options
        .iter()
        .map(|choice| {
            let suffix = if choice.default_value.is_empty() {
                String::new()
            } else {
                format!("  [default: {}]", choice.default_value)
            };
            ListItem::new(Line::from(vec![
                Span::styled(choice.label.clone(), Style::default().fg(Color::White)),
                Span::styled(suffix, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(picker.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Add binding parameter")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_parameter_editor(frame: &mut Frame, parent: Rect, modal: &SourceBindingModal) {
    let Some(editor) = modal.parameter_editor.as_ref() else {
        return;
    };
    let area = centered_rect(70, 46, parent);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(if editor.original_name.is_some() {
            "Edit binding parameter"
        } else {
            "Add binding parameter"
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);
    let cursor = "█";
    let name = if editor.selected_field == 0 {
        format!("{}{cursor}", editor.name)
    } else {
        editor.name.clone()
    };
    let value = if editor.selected_field == 1 {
        format!("{}{cursor}", editor.value)
    } else {
        editor.value.clone()
    };
    let mut state = ListState::default();
    state.select(Some(editor.selected_field));
    let list = List::new(vec![
        ListItem::new(Line::from(vec![
            Span::styled("Name   ", Style::default().fg(Color::Gray)),
            Span::styled(name, Style::default().fg(Color::Cyan)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("Value  ", Style::default().fg(Color::Gray)),
            Span::styled(value, Style::default().fg(Color::Magenta)),
        ])),
    ])
    .block(Block::default().title("Fields").borders(Borders::ALL))
    .highlight_symbol("▶ ")
    .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, rows[0], &mut state);

    let status = editor
        .error
        .as_deref()
        .unwrap_or("Prefix with json: for numbers, booleans, arrays, or objects");
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(if editor.error.is_some() {
                Color::Red
            } else {
                Color::Gray
            }))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rows[1],
    );
}

fn render_mode_picker(frame: &mut Frame, parent: Rect, modal: &SourceBindingModal) {
    let Some(picker) = modal.mode_picker.as_ref() else {
        return;
    };
    let area = centered_rect(42, 54, parent);
    frame.render_widget(Clear, area);

    let items = picker
        .options
        .iter()
        .map(|mode| ListItem::new(mode.as_str().to_owned()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(picker.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Select Mode")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_action_picker(
    frame: &mut Frame,
    parent: Rect,
    modal: &SourceBindingModal,
    actions: &BTreeMap<String, ActionType>,
) {
    let Some(picker) = modal.action_picker.as_ref() else {
        return;
    };
    let area = centered_rect(68, 72, parent);
    frame.render_widget(Clear, area);

    let items = picker
        .options
        .iter()
        .map(|option| match option {
            None => ListItem::new(Line::from(Span::styled(
                "None",
                Style::default().fg(Color::DarkGray),
            ))),
            Some(path) => {
                let kind = find_action(actions, path)
                    .map(|(_, kind)| kind.as_str())
                    .unwrap_or("unknown");
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<24}", action_display_name(actions, path)),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(format!(" [{kind}]"), Style::default().fg(Color::DarkGray)),
                ]))
            }
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(picker.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Select action for {}", picker.input.as_str()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_other_binding_modal(
    frame: &mut Frame,
    screen_area: Rect,
    modal: &mut OtherBindingModal,
    actions: &BTreeMap<String, ActionType>,
    path_sources: &[PathSourceInfo],
    chord_sources: &[ChordSourceInfo],
) {
    match modal {
        OtherBindingModal::Path(modal) => {
            render_path_binding_modal(frame, screen_area, modal, actions, path_sources)
        }
        OtherBindingModal::Chord(modal) => {
            render_chord_binding_modal(frame, screen_area, modal, actions, chord_sources)
        }
    }
}

fn render_path_binding_modal(
    frame: &mut Frame,
    screen_area: Rect,
    modal: &PathBindingModal,
    actions: &BTreeMap<String, ActionType>,
    sources: &[PathSourceInfo],
) {
    let area = centered_rect(78, 58, screen_area);
    frame.render_widget(Clear, area);
    let title = format!(
        " {} {} binding ",
        if modal.original_index.is_some() {
            "Edit"
        } else {
            "Add"
        },
        modal.kind.display_name()
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let source_name = path_source_display_name(sources, modal.kind, &modal.draft.path);
    let mut state = ListState::default();
    state.select(Some(modal.selected_field));
    let list = List::new(vec![
        ListItem::new(Line::from(vec![
            Span::styled("Source  ", Style::default().fg(Color::Gray)),
            Span::styled(source_name, Style::default().fg(Color::Cyan)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("Action  ", Style::default().fg(Color::Gray)),
            Span::styled(
                action_display_name(actions, &modal.draft.output),
                Style::default().fg(Color::Green),
            ),
        ])),
    ])
    .block(Block::default().title("Fields").borders(Borders::ALL))
    .highlight_symbol("▶ ")
    .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, rows[0], &mut state);

    let status = modal
        .error
        .as_deref()
        .unwrap_or("↑/↓ field • ←/→ cycle • Enter choose • s apply • d/Del delete • Esc cancel");
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(if modal.error.is_some() {
                Color::Red
            } else {
                Color::Gray
            }))
            .alignment(Alignment::Center),
        rows[1],
    );

    if let Some(picker) = modal
        .source_picker
        .as_ref()
        .or(modal.action_picker.as_ref())
    {
        render_string_picker(frame, area, picker);
    }
}

fn render_chord_binding_modal(
    frame: &mut Frame,
    screen_area: Rect,
    modal: &ChordBindingModal,
    actions: &BTreeMap<String, ActionType>,
    sources: &[ChordSourceInfo],
) {
    let area = centered_rect(84, 76, screen_area);
    frame.render_widget(Clear, area);
    let title = if modal.original_index.is_some() {
        " Edit Chord "
    } else {
        " Add Chord "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(2)])
        .split(inner);

    let mut items = vec![ListItem::new(Line::from(vec![
        Span::styled("Action       ", Style::default().fg(Color::Gray)),
        Span::styled(
            action_display_name(actions, &modal.draft.output),
            Style::default().fg(Color::Green),
        ),
    ]))];
    for (index, input) in modal.draft.inputs.iter().enumerate() {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("Input #{}     ", index + 1),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                chord_source_display_name(sources, &input.0),
                Style::default().fg(Color::Cyan),
            ),
        ])));
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  Activator  ", Style::default().fg(Color::Gray)),
            Span::styled(
                input.1.as_str().to_owned(),
                Style::default().fg(Color::Magenta),
            ),
        ])));
    }
    items.push(ListItem::new(Line::from(Span::styled(
        "[+] Add chord input",
        Style::default().fg(Color::Blue),
    ))));

    let mut state = ListState::default();
    state.select(Some(modal.selected_field));
    let list = List::new(items)
        .block(Block::default().title("Chord").borders(Borders::ALL))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, rows[0], &mut state);

    let status = modal.error.as_deref().unwrap_or(
        "↑/↓ field • ←/→ cycle • Enter choose • n add input • x/Del remove input • s apply • d delete",
    );
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(if modal.error.is_some() {
                Color::Red
            } else {
                Color::Gray
            }))
            .alignment(Alignment::Center),
        rows[1],
    );

    if let Some(picker) = modal.action_picker.as_ref() {
        render_string_picker(frame, area, picker);
    } else if let Some((_, picker)) = modal.source_picker.as_ref() {
        render_string_picker(frame, area, picker);
    } else if let Some(picker) = modal.kind_picker.as_ref() {
        render_chord_kind_picker(frame, area, picker);
    }
}

fn render_string_picker(frame: &mut Frame, parent: Rect, picker: &StringPicker) {
    let area = centered_rect(68, 70, parent);
    frame.render_widget(Clear, area);
    let items = picker
        .options
        .iter()
        .map(|choice| ListItem::new(choice.label.clone()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(picker.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title(picker.title.clone())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_chord_kind_picker(frame: &mut Frame, parent: Rect, picker: &ChordKindPicker) {
    let area = centered_rect(42, 52, parent);
    frame.render_widget(Clear, area);
    let items = picker
        .options
        .iter()
        .map(|kind| ListItem::new(kind.as_str().to_owned()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(picker.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title("Select chord activator")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, area, &mut state);
}

fn known_parameter_specs(def: &InputSourceDef, mode: &BindingMode) -> Vec<ParameterSpec> {
    fn spec(
        name: &'static str,
        label: &'static str,
        default_value: &str,
        choices: &[&str],
    ) -> ParameterSpec {
        ParameterSpec {
            name,
            label,
            default_value: default_value.to_owned(),
            choices: choices.iter().map(|choice| (*choice).to_owned()).collect(),
        }
    }

    let mut result = Vec::new();
    let mut scalar_inputs = Vec::new();
    if def.click {
        scalar_inputs.push("click");
    }
    if def.touch {
        scalar_inputs.push("touch");
    }
    if def.value {
        scalar_inputs.push("value");
    }
    if def.force {
        scalar_inputs.push("force");
    }

    match mode {
        BindingMode::Dpad => {
            result.push(spec("deadzone_pct", "Deadzone percent", "25", &[]));
            result.push(spec("overlap_pct", "Direction overlap percent", "0", &[]));
            let mut sub_modes = Vec::new();
            if def.touch {
                sub_modes.push("touch");
            }
            if def.click {
                sub_modes.push("click");
            }
            if sub_modes.is_empty() {
                sub_modes.extend(["touch", "click"]);
            }
            result.push(spec(
                "sub_mode",
                "Activation component",
                sub_modes[0],
                &sub_modes,
            ));
        }
        BindingMode::Button | BindingMode::ToggleButton | BindingMode::Trigger => {
            if scalar_inputs.len() > 1 || def.value || def.force {
                let default = if def.force {
                    "force"
                } else if def.value {
                    "value"
                } else if def.click {
                    "click"
                } else {
                    "touch"
                };
                result.push(spec(
                    "force_input",
                    "Physical component",
                    default,
                    &scalar_inputs,
                ));
            }
            if def.value || def.force {
                result.push(spec(
                    "click_activate_threshold",
                    "Click activation threshold",
                    "0.8",
                    &[],
                ));
                result.push(spec(
                    "click_deactivate_threshold",
                    "Click release threshold",
                    "0.7",
                    &[],
                ));
                result.push(spec(
                    "touch_activate_threshold",
                    "Touch activation threshold",
                    "0.1",
                    &[],
                ));
                result.push(spec(
                    "touch_deactivate_threshold",
                    "Touch release threshold",
                    "0.05",
                    &[],
                ));
            }
            result.push(spec(
                "haptic_amplitude",
                "Activation haptic amplitude",
                "0.0",
                &[],
            ));
        }
        BindingMode::Grab => {
            result.push(spec(
                "force_hold_threshold",
                "Grab hold threshold",
                "0.02",
                &[],
            ));
            result.push(spec(
                "force_release_threshold",
                "Grab release threshold",
                "0.01",
                &[],
            ));
        }
        BindingMode::Joystick | BindingMode::Trackpad => {
            result.push(spec("deadzone_pct", "Deadzone percent", "0", &[]));
            result.push(spec("maxzone_pct", "Maximum zone percent", "100", &[]));
            result.push(spec(
                "invert",
                "Axis inversion",
                "n",
                &["n", "x", "y", "xy"],
            ));
        }
        BindingMode::Scroll | BindingMode::ScalarConstant | BindingMode::Other(_) => {}
    }

    result
}

fn parameter_value_edit_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        _ => format!("json:{value}"),
    }
}

fn parse_parameter_value(value: &str) -> Result<serde_json::Value, String> {
    if let Some(json) = value.strip_prefix("json:") {
        serde_json::from_str(json).map_err(|error| format!("Invalid JSON value: {error}"))
    } else {
        Ok(serde_json::Value::String(value.to_owned()))
    }
}

fn compatible_action_options(
    actions: &BTreeMap<String, ActionType>,
    action_set_name: &str,
    mode: &BindingMode,
    input: &BindingInput,
    current: Option<&str>,
) -> Vec<Option<String>> {
    let prefix = format!("{}/in/", action_set_name.trim_end_matches('/'));
    let required = required_action_type(mode, input);
    let mut options = vec![None];

    for (name, action_type) in actions {
        if !starts_with_ignore_ascii_case(name, &prefix) {
            continue;
        }
        if required
            .map(|required| action_type_matches(required, action_type))
            .unwrap_or(true)
        {
            // store the exact spelling from actions.json
            options.push(Some(name.clone()));
        }
    }

    if let Some(current) = current {
        if !options.iter().any(|option| {
            option
                .as_deref()
                .is_some_and(|option| option.eq_ignore_ascii_case(current))
        }) {
            // preserve an unknown action from an existing binding
            options.push(Some(current.to_owned()));
        }
    }

    options
}

fn action_choices(
    actions: &BTreeMap<String, ActionType>,
    action_set_name: &str,
    required_type: &ActionType,
    direction: ActionDirection,
    current: Option<&str>,
) -> Vec<StringChoice> {
    let direction = match direction {
        ActionDirection::Input => "in",
        ActionDirection::Output => "out",
    };
    let prefix = format!("{}/{direction}/", action_set_name.trim_end_matches('/'));
    let mut choices = actions
        .iter()
        .filter(|(name, kind)| {
            starts_with_ignore_ascii_case(name, &prefix) && same_action_type(kind, required_type)
        })
        .map(|(name, _)| StringChoice {
            value: name.clone(),
            label: action_display_name(actions, name),
        })
        .collect::<Vec<_>>();

    if let Some(current) = current.filter(|current| !current.is_empty()) {
        add_current_choice(
            &mut choices,
            current,
            &action_display_name(actions, current),
        );
    }
    choices
}

fn same_action_type(left: &ActionType, right: &ActionType) -> bool {
    matches!(
        (left, right),
        (ActionType::Boolean, ActionType::Boolean)
            | (ActionType::Vector1, ActionType::Vector1)
            | (ActionType::Vector2, ActionType::Vector2)
            | (ActionType::Vector3, ActionType::Vector3)
            | (ActionType::Pose, ActionType::Pose)
            | (ActionType::Skeleton, ActionType::Skeleton)
            | (ActionType::Vibration, ActionType::Vibration)
    ) || matches!((left, right), (ActionType::Other(a), ActionType::Other(b)) if a == b)
}

fn add_current_choice(choices: &mut Vec<StringChoice>, value: &str, label: &str) {
    if value.is_empty()
        || choices
            .iter()
            .any(|choice| choice.value.eq_ignore_ascii_case(value))
    {
        return;
    }
    choices.push(StringChoice {
        value: value.to_owned(),
        label: label.to_owned(),
    });
}

fn choice_index(choices: &[StringChoice], value: &str) -> usize {
    choices
        .iter()
        .position(|choice| choice.value.eq_ignore_ascii_case(value))
        .unwrap_or(0)
}

fn cycle_choice_value(value: &mut String, choices: &[StringChoice], forward: bool) {
    if choices.is_empty() {
        return;
    }
    let index = choice_index(choices, value);
    *value = choices[step_index(index, choices.len(), forward)]
        .value
        .clone();
}

fn chord_source_choices(sources: &[ChordSourceInfo], current: Option<&str>) -> Vec<StringChoice> {
    let mut choices = sources
        .iter()
        .map(|source| StringChoice {
            value: source.path.clone(),
            label: source.name.clone(),
        })
        .collect::<Vec<_>>();
    if let Some(current) = current.filter(|current| !current.is_empty()) {
        add_current_choice(&mut choices, current, short_path(current));
    }
    choices
}

fn chord_kind_options(current: &ChordInputKind) -> Vec<ChordInputKind> {
    let mut options = vec![
        ChordInputKind::Held,
        ChordInputKind::Single,
        ChordInputKind::Click,
        ChordInputKind::Touch,
    ];
    if !options.contains(current) {
        options.push(current.clone());
    }
    options
}

fn is_toggle_button_input(input: &BindingInput) -> bool {
    matches!(
        input,
        BindingInput::Toggle | BindingInput::Click | BindingInput::Touch
    )
}

fn supported_modes(def: &InputSourceDef) -> Vec<BindingMode> {
    let mut modes = match &def.r#type {
        InputSourceType::Button => vec![BindingMode::Button],
        InputSourceType::Trigger | InputSourceType::Pinch => {
            vec![BindingMode::Button, BindingMode::Trigger]
        }
        InputSourceType::Joystick => vec![
            BindingMode::Button,
            BindingMode::Joystick,
            BindingMode::Dpad,
            BindingMode::Scroll,
        ],
        InputSourceType::Trackpad => vec![
            BindingMode::Button,
            BindingMode::Trackpad,
            BindingMode::Dpad,
            BindingMode::Scroll,
        ],
        InputSourceType::Pose | InputSourceType::Skeleton | InputSourceType::Vibration => {
            Vec::new()
        }
    };

    if def.supports_toggle_button() {
        modes.insert(1.min(modes.len()), BindingMode::ToggleButton);
    }

    modes
}

fn default_binding_path(
    profile_path: &str,
    side: ControllerSide,
    ui_mode: &BindingUiMode,
    binding: &SteamVrBindingFile,
) -> String {
    if profile_path.starts_with("/user/") {
        return profile_path.to_owned();
    }

    let root = match side {
        ControllerSide::Left => Some("/user/hand/left".to_owned()),
        ControllerSide::Right => Some("/user/hand/right".to_owned()),
        ControllerSide::Single => match ui_mode {
            BindingUiMode::Hmd => Some("/user/head".to_owned()),
            BindingUiMode::SingleDevice | BindingUiMode::Other(_) => {
                infer_single_device_user_root(binding)
            }
            BindingUiMode::ControllerHanded => None,
        },
    };

    match root {
        Some(root) => {
            let suffix = if profile_path.starts_with('/') {
                profile_path.to_owned()
            } else {
                format!("/{profile_path}")
            };
            format!("{}{suffix}", root.trim_end_matches('/'))
        }
        None => profile_path.to_owned(),
    }
}

fn infer_single_device_user_root(binding: &SteamVrBindingFile) -> Option<String> {
    fn root_from_path(path: &str) -> Option<String> {
        ["/input/", "/pose/", "/output/"]
            .into_iter()
            .filter_map(|marker| path.find(marker))
            .min()
            .map(|index| path[..index].to_owned())
    }

    for bindings in binding.bindings.values() {
        if let Some(root) = bindings
            .sources()
            .iter()
            .find_map(|source| root_from_path(&source.path))
        {
            return Some(root);
        }

        for path_binding in bindings
            .poses
            .as_deref()
            .unwrap_or_default()
            .iter()
            .chain(bindings.skeleton.as_deref().unwrap_or_default())
            .chain(bindings.haptics.as_deref().unwrap_or_default())
        {
            if let Some(root) = root_from_path(&path_binding.path) {
                return Some(root);
            }
        }

        if let Some(root) = bindings
            .chords
            .as_deref()
            .unwrap_or_default()
            .iter()
            .flat_map(|chord| &chord.inputs)
            .find_map(|input| root_from_path(&input.0))
        {
            return Some(root);
        }
    }

    None
}

fn step_index(index: usize, len: usize, forward: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if forward {
        (index + 1) % len
    } else if index == 0 {
        len - 1
    } else {
        index - 1
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_save_path_dialog(frame: &mut Frame, screen_area: Rect, dialog: &SavePathDialog) {
    let area = centered_rect(50, 30, screen_area);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Compat library to save for ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let options = dialog.options();
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if i == dialog.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(*text, style))
        })
        .collect();

    let list = List::new(items).block(Block::default());

    frame.render_widget(list, inner);
}

fn render_discard_dialog(frame: &mut Frame, screen_area: Rect) {
    let area = centered_rect(50, 20, screen_area);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = Paragraph::new(Line::from(vec![
        Span::styled(
            "Discard unsaved changes?",
            Style::default().fg(Color::White),
        ),
        Span::styled(
            " [Y]es ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "[N]o ",
            Style::default().fg(Color::White),
        ),
    ]))
    .alignment(Alignment::Center);

    frame.render_widget(text, inner);
}

fn empty_binding(controller_type: String) -> SteamVrBindingFile {
    SteamVrBindingFile {
        action_manifest_version: None,
        app_key: None,
        controller_type,
        name: String::new(),
        description: String::new(),
        bindings: BTreeMap::new(),
        category: None,
        alias_info: None,
        options: None,
        simulated_actions: None,
        extra: BTreeMap::new(),
    }
}

fn load_binding_file(path: PathBuf) -> Result<(SteamVrBindingFile, PathBuf), String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read binding {}: {error}", path.display()))?;
    let binding = serde_json::from_str::<SteamVrBindingFile>(&content)
        .map_err(|error| format!("Failed to parse binding {}: {error}", path.display()))?;

    Ok((binding, path))
}

fn load_manifest_binding(
    manifest: &ActionManifest,
    controller_type: &str,
    actions_json_path: &Path,
) -> Result<Option<(SteamVrBindingFile, PathBuf)>, String> {
    let Some(default_binding) = manifest.default_bindings.iter().find(|binding| {
        binding
            .controller_type
            .eq_ignore_ascii_case(controller_type)
    }) else {
        return Ok(None);
    };

    let binding_path = resolve_binding_path(actions_json_path, &default_binding.binding_url);
    load_binding_file(binding_path).map(Some)
}

fn resolve_binding_path(actions_json_path: &Path, binding_url: &str) -> PathBuf {
    let binding_url = binding_url.strip_prefix("file://").unwrap_or(binding_url);
    let binding_path = PathBuf::from(binding_url);

    if binding_path.is_absolute() {
        binding_path
    } else {
        actions_json_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(binding_path)
    }
}

fn action_set_names_match(left: &str, right: &str) -> bool {
    left.trim_end_matches('/')
        .eq_ignore_ascii_case(right.trim_end_matches('/'))
}

fn find_action_set_bindings<'a>(
    bindings: &'a BTreeMap<String, ActionSetBindings>,
    action_set_name: &str,
) -> Option<&'a ActionSetBindings> {
    bindings.get(action_set_name).or_else(|| {
        bindings
            .iter()
            .find(|(name, _)| action_set_names_match(name, action_set_name))
            .map(|(_, bindings)| bindings)
    })
}

fn resolved_action_set_name(
    bindings: &BTreeMap<String, ActionSetBindings>,
    action_set_name: &str,
) -> String {
    bindings
        .get_key_value(action_set_name)
        .map(|(name, _)| name.clone())
        .or_else(|| {
            bindings
                .keys()
                .find(|name| action_set_names_match(name, action_set_name))
                .cloned()
        })
        .unwrap_or_else(|| action_set_name.to_owned())
}

fn collect_source(
    path: &str,
    def: &InputSourceDef,
    side: Option<&str>,
    ui_mode: &BindingUiMode,
    left_sources: &mut Vec<SourceInfo>,
    right_sources: &mut Vec<SourceInfo>,
) {
    if matches!(
        &def.r#type,
        InputSourceType::Pose | InputSourceType::Skeleton | InputSourceType::Vibration
    ) {
        return;
    }

    let source = SourceInfo {
        path: path.to_owned(),
        name: source_name_from_path(path),
        def: def.clone(),
    };

    match ui_mode {
        BindingUiMode::ControllerHanded => match side {
            Some("left") => left_sources.push(source),
            Some("right") => right_sources.push(source),
            _ => {
                left_sources.push(source.clone());
                right_sources.push(source);
            }
        },
        BindingUiMode::Hmd | BindingUiMode::SingleDevice | BindingUiMode::Other(_) => {
            left_sources.push(source)
        }
    }

    if let Some(children) = &def.input_source {
        for (child_name, child_def) in children {
            let child_path = if child_name.starts_with('/') {
                child_name.clone()
            } else {
                format!(
                    "{}/{}",
                    path.trim_end_matches('/'),
                    child_name.trim_start_matches('/')
                )
            };
            collect_source(
                &child_path,
                child_def,
                child_def.side.as_deref().or(side),
                ui_mode,
                left_sources,
                right_sources,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_aux_sources(
    path: &str,
    def: &InputSourceDef,
    side: Option<&str>,
    ui_mode: &BindingUiMode,
    binding: &SteamVrBindingFile,
    path_sources: &mut Vec<PathSourceInfo>,
    chord_sources: &mut Vec<ChordSourceInfo>,
) {
    let sides = source_sides(side, ui_mode);
    for controller_side in sides {
        let full_path = default_binding_path(path, controller_side, ui_mode, binding);
        let name = profile_source_display_name(path, controller_side, ui_mode);
        match &def.r#type {
            InputSourceType::Pose => path_sources.push(PathSourceInfo {
                path: full_path,
                name,
                kind: PathBindingKind::Pose,
            }),
            InputSourceType::Skeleton => path_sources.push(PathSourceInfo {
                path: full_path,
                name,
                kind: PathBindingKind::Skeleton,
            }),
            InputSourceType::Vibration => path_sources.push(PathSourceInfo {
                path: full_path,
                name,
                kind: PathBindingKind::Haptic,
            }),
            InputSourceType::Button
            | InputSourceType::Trigger
            | InputSourceType::Joystick
            | InputSourceType::Trackpad
            | InputSourceType::Pinch => {
                if !is_reserved_system_source(path) {
                    chord_sources.push(ChordSourceInfo {
                        path: full_path,
                        name,
                    });
                }
            }
        }
    }

    if let Some(children) = &def.input_source {
        for (child_name, child_def) in children {
            let child_path = if child_name.starts_with('/') {
                child_name.clone()
            } else {
                format!(
                    "{}/{}",
                    path.trim_end_matches('/'),
                    child_name.trim_start_matches('/')
                )
            };
            collect_aux_sources(
                &child_path,
                child_def,
                child_def.side.as_deref().or(side),
                ui_mode,
                binding,
                path_sources,
                chord_sources,
            );
        }
    }
}

fn source_sides(side: Option<&str>, ui_mode: &BindingUiMode) -> Vec<ControllerSide> {
    match ui_mode {
        BindingUiMode::ControllerHanded => match side {
            Some(side) if side.eq_ignore_ascii_case("left") => vec![ControllerSide::Left],
            Some(side) if side.eq_ignore_ascii_case("right") => vec![ControllerSide::Right],
            _ => vec![ControllerSide::Left, ControllerSide::Right],
        },
        BindingUiMode::Hmd | BindingUiMode::SingleDevice | BindingUiMode::Other(_) => {
            vec![ControllerSide::Single]
        }
    }
}

fn profile_source_display_name(
    path: &str,
    side: ControllerSide,
    ui_mode: &BindingUiMode,
) -> String {
    let name = source_name_from_path(path);
    if !matches!(ui_mode, BindingUiMode::ControllerHanded) {
        return name;
    }
    match side {
        ControllerSide::Left => format!("Left {name}"),
        ControllerSide::Right => format!("Right {name}"),
        ControllerSide::Single => name,
    }
}

fn path_binding_kind_order(kind: PathBindingKind) -> u8 {
    match kind {
        PathBindingKind::Pose => 0,
        PathBindingKind::Skeleton => 1,
        PathBindingKind::Haptic => 2,
    }
}

fn dedup_path_sources(sources: &mut Vec<PathSourceInfo>) {
    let mut result = Vec::with_capacity(sources.len());
    for source in sources.drain(..) {
        if !result.iter().any(|existing: &PathSourceInfo| {
            existing.kind == source.kind && existing.path.eq_ignore_ascii_case(&source.path)
        }) {
            result.push(source);
        }
    }
    *sources = result;
}

fn dedup_chord_sources(sources: &mut Vec<ChordSourceInfo>) {
    let mut result = Vec::with_capacity(sources.len());
    for source in sources.drain(..) {
        if !result
            .iter()
            .any(|existing: &ChordSourceInfo| existing.path.eq_ignore_ascii_case(&source.path))
        {
            result.push(source);
        }
    }
    *sources = result;
}

fn is_reserved_system_source(path: &str) -> bool {
    path.eq_ignore_ascii_case("/input/system") || ends_with_ignore_ascii_case(path, "/input/system")
}

fn side_from_map_key(key: &str) -> Option<&'static str> {
    if key.eq_ignore_ascii_case("left")
        || key.eq_ignore_ascii_case("lefthand")
        || key.eq_ignore_ascii_case("left_hand")
    {
        Some("left")
    } else if key.eq_ignore_ascii_case("right")
        || key.eq_ignore_ascii_case("righthand")
        || key.eq_ignore_ascii_case("right_hand")
    {
        Some("right")
    } else {
        None
    }
}

fn source_list_item(
    source: &SourceInfo,
    side: ControllerSide,
    bindings: &[SourceBinding],
    actions: &BTreeMap<String, ActionType>,
) -> ListItem<'static> {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            source.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  [{}]", input_source_type_name(&source.def.r#type)),
            Style::default().fg(Color::Gray),
        ),
    ])];

    let matching: Vec<&SourceBinding> = bindings
        .iter()
        .filter(|binding| source_binding_matches(&binding.path, &source.path, side))
        .collect();

    if matching.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (unbound)",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for (index, binding) in matching.iter().enumerate() {
            let number = if matching.len() > 1 {
                format!(" #{}", index + 1)
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::styled("  USE AS ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    binding.mode.as_str().to_ascii_uppercase(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(number, Style::default().fg(Color::Cyan)),
            ]));

            for input in display_inputs(&source.def, binding) {
                let output = binding.inputs.get(&input);
                let (output_name, output_style) = match output {
                    Some(output) => {
                        let style =
                            action_output_style(&binding.mode, &input, &output.output, actions);
                        (action_display_name(actions, &output.output), style)
                    }
                    None => ("None".to_owned(), Style::default().fg(Color::DarkGray)),
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<10}", input.as_str()),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(output_name, output_style),
                ]));
            }

            if let Some(parameters) = binding.parameters.as_ref().filter(|p| !p.is_empty()) {
                let parameters = parameters
                    .iter()
                    .map(|(name, value)| format!("{name}={}", compact_json_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(Line::from(vec![
                    Span::styled("    params    ", Style::default().fg(Color::White)),
                    Span::styled(parameters, Style::default().fg(Color::Gray)),
                ]));
            }
        }
    }

    ListItem::new(lines)
}

fn display_inputs(def: &InputSourceDef, binding: &SourceBinding) -> Vec<BindingInput> {
    let mut inputs =
        if matches!(&binding.mode, BindingMode::ToggleButton) && !binding.inputs.is_empty() {
            Vec::new()
        } else {
            expected_inputs(def, &binding.mode)
        };

    // preserve and display fields from modes that we do not fully understand
    for input in binding.inputs.keys() {
        if !inputs.contains(input) {
            inputs.push(input.clone());
        }
    }

    inputs
}

fn expected_inputs(def: &InputSourceDef, mode: &BindingMode) -> Vec<BindingInput> {
    let mut inputs = Vec::new();

    match mode {
        BindingMode::ToggleButton => {
            inputs.push(BindingInput::Toggle);
        }
        BindingMode::Button => {
            if def.click || def.value || def.force {
                inputs.push(BindingInput::Click);
            }
            if def.touch {
                inputs.push(BindingInput::Touch);
            }
        }
        BindingMode::Trigger => {
            if def.value || def.force {
                inputs.push(BindingInput::Pull);
            }
            if def.click {
                inputs.push(BindingInput::Click);
            }
            if def.touch {
                inputs.push(BindingInput::Touch);
            }
        }
        BindingMode::Joystick | BindingMode::Trackpad => {
            inputs.push(BindingInput::Position);
            if def.click {
                inputs.push(BindingInput::Click);
            }
            if def.touch {
                inputs.push(BindingInput::Touch);
            }
            if def.force {
                inputs.push(BindingInput::Force);
            }
        }
        BindingMode::Dpad => {
            inputs.extend([
                BindingInput::North,
                BindingInput::South,
                BindingInput::East,
                BindingInput::West,
                BindingInput::Center,
            ]);
        }
        BindingMode::Grab => {
            inputs.push(BindingInput::Grab);
            if def.value {
                inputs.push(BindingInput::Value);
            }
            if def.force {
                inputs.push(BindingInput::Force);
            }
            if def.touch {
                inputs.push(BindingInput::Touch);
            }
        }
        BindingMode::ScalarConstant => inputs.push(BindingInput::Value),
        BindingMode::Scroll | BindingMode::Other(_) => {}
    }

    inputs
}

fn source_binding_matches(binding_path: &str, profile_path: &str, side: ControllerSide) -> bool {
    // existing bindings are often normalized to lowercase
    let path_matches = binding_path.eq_ignore_ascii_case(profile_path)
        || ends_with_ignore_ascii_case(binding_path, profile_path);
    if !path_matches {
        return false;
    }

    match side {
        ControllerSide::Left => {
            contains_ignore_ascii_case(binding_path, "/left/")
                || starts_with_ignore_ascii_case(binding_path, "/user/hand/left")
        }
        ControllerSide::Right => {
            contains_ignore_ascii_case(binding_path, "/right/")
                || starts_with_ignore_ascii_case(binding_path, "/user/hand/right")
        }
        ControllerSide::Single => true,
    }
}

fn action_output_style(
    mode: &BindingMode,
    input: &BindingInput,
    output: &str,
    actions: &BTreeMap<String, ActionType>,
) -> Style {
    let Some((_, action_type)) = find_action(actions, output) else {
        return Style::default().fg(Color::Yellow);
    };
    let Some(required) = required_action_type(mode, input) else {
        return Style::default().fg(Color::Green);
    };

    if action_type_matches(required, action_type) {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }
}

fn required_action_type(mode: &BindingMode, input: &BindingInput) -> Option<RequiredActionType> {
    match (mode, input) {
        (BindingMode::ToggleButton, input) if is_toggle_button_input(input) => {
            Some(RequiredActionType::Boolean)
        }
        (BindingMode::Trigger, BindingInput::Pull)
        | (BindingMode::Grab, BindingInput::Value)
        | (BindingMode::Grab, BindingInput::Force)
        | (BindingMode::Joystick, BindingInput::Force)
        | (BindingMode::Trackpad, BindingInput::Force)
        | (BindingMode::ScalarConstant, BindingInput::Value) => Some(RequiredActionType::Vector1),

        (BindingMode::Joystick, BindingInput::Position)
        | (BindingMode::Trackpad, BindingInput::Position) => Some(RequiredActionType::Vector2),

        (BindingMode::Button, BindingInput::Click | BindingInput::Touch)
        | (BindingMode::Trigger, BindingInput::Click | BindingInput::Touch)
        | (BindingMode::Joystick, BindingInput::Click | BindingInput::Touch)
        | (BindingMode::Trackpad, BindingInput::Click | BindingInput::Touch)
        | (BindingMode::Dpad, _)
        | (BindingMode::Grab, BindingInput::Grab | BindingInput::Touch) => {
            Some(RequiredActionType::Boolean)
        }

        _ => None,
    }
}

fn action_type_matches(required: RequiredActionType, actual: &ActionType) -> bool {
    matches!(
        (required, actual),
        (RequiredActionType::Boolean, ActionType::Boolean)
            | (RequiredActionType::Vector1, ActionType::Vector1)
            | (RequiredActionType::Vector2, ActionType::Vector2)
    )
}

fn path_bindings(bindings: &ActionSetBindings, kind: PathBindingKind) -> &[PathBinding] {
    match kind {
        PathBindingKind::Pose => bindings.poses.as_deref().unwrap_or_default(),
        PathBindingKind::Skeleton => bindings.skeleton.as_deref().unwrap_or_default(),
        PathBindingKind::Haptic => bindings.haptics.as_deref().unwrap_or_default(),
    }
}

fn path_bindings_mut(
    bindings: &mut ActionSetBindings,
    kind: PathBindingKind,
) -> &mut Vec<PathBinding> {
    match kind {
        PathBindingKind::Pose => bindings.poses_mut(),
        PathBindingKind::Skeleton => bindings.skeleton_mut(),
        PathBindingKind::Haptic => bindings.haptics_mut(),
    }
}

fn other_binding_list_item(
    entry: &OtherBindingEntry,
    bindings: &ActionSetBindings,
    actions: &BTreeMap<String, ActionType>,
    path_sources: &[PathSourceInfo],
    chord_sources: &[ChordSourceInfo],
) -> ListItem<'static> {
    match entry {
        OtherBindingEntry::Path { kind, index } => {
            let Some(binding) = path_bindings(bindings, *kind).get(*index) else {
                return ListItem::new("invalid binding");
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<10}", kind.display_name()),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    action_display_name(actions, &binding.output),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(" <- ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    path_source_display_name(path_sources, *kind, &binding.path),
                    Style::default().fg(Color::White),
                ),
            ]))
        }
        OtherBindingEntry::AddPath(kind) => ListItem::new(Line::from(Span::styled(
            format!(
                "[+] Add {} binding",
                kind.display_name().to_ascii_lowercase()
            ),
            Style::default().fg(Color::Blue),
        ))),
        OtherBindingEntry::Chord { index } => {
            let Some(chord) = bindings
                .chords
                .as_deref()
                .and_then(|chords| chords.get(*index))
            else {
                return ListItem::new("invalid chord");
            };
            let inputs = chord
                .inputs
                .iter()
                .map(|input| {
                    format!(
                        "{} {}",
                        chord_source_display_name(chord_sources, &input.0),
                        input.1.as_str()
                    )
                })
                .collect::<Vec<_>>()
                .join(" + ");
            ListItem::new(Line::from(vec![
                Span::styled(
                    "Chord     ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    action_display_name(actions, &chord.output),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(" <- ", Style::default().fg(Color::DarkGray)),
                Span::raw(inputs),
            ]))
        }
        OtherBindingEntry::AddChord => ListItem::new(Line::from(Span::styled(
            "[+] Add chord",
            Style::default().fg(Color::Blue),
        ))),
    }
}

fn path_source_display_name(
    sources: &[PathSourceInfo],
    kind: PathBindingKind,
    path: &str,
) -> String {
    sources
        .iter()
        .find(|source| source.kind == kind && source.path.eq_ignore_ascii_case(path))
        .map(|source| source.name.clone())
        .unwrap_or_else(|| short_path(path).to_owned())
}

fn chord_source_display_name(sources: &[ChordSourceInfo], path: &str) -> String {
    sources
        .iter()
        .find(|source| source.path.eq_ignore_ascii_case(path))
        .map(|source| source.name.clone())
        .unwrap_or_else(|| short_path(path).to_owned())
}

fn find_action<'a>(
    actions: &'a BTreeMap<String, ActionType>,
    path: &str,
) -> Option<(&'a String, &'a ActionType)> {
    actions
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(path))
}

/// Returns the action's display spelling from actions.json
fn action_display_name(actions: &BTreeMap<String, ActionType>, path: &str) -> String {
    let canonical = find_action(actions, path)
        .map(|(name, _)| name.as_str())
        .unwrap_or(path);
    short_path(canonical).to_owned()
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn ends_with_ignore_ascii_case(value: &str, suffix: &str) -> bool {
    value
        .len()
        .checked_sub(suffix.len())
        .and_then(|start| value.get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
}

fn contains_ignore_ascii_case(value: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    value
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn compact_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

fn input_source_type_name(kind: &InputSourceType) -> &'static str {
    match kind {
        InputSourceType::Button => "button",
        InputSourceType::Trigger => "trigger",
        InputSourceType::Joystick => "joystick",
        InputSourceType::Trackpad => "trackpad",
        InputSourceType::Pose => "pose",
        InputSourceType::Skeleton => "skeleton",
        InputSourceType::Vibration => "vibration",
        InputSourceType::Pinch => "pinch",
    }
}

fn source_name_from_path(path: &str) -> String {
    short_path(path).replace('_', " ")
}

fn short_path(path: &str) -> &str {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_set_lookup_ignores_case_and_trailing_slashes() {
        let mut bindings = BTreeMap::new();
        bindings.insert(
            "/actions/Main".to_owned(),
            ActionSetBindings::default(),
        );

        assert!(find_action_set_bindings(&bindings, "/actions/main/").is_some());
        assert_eq!(
            resolved_action_set_name(&bindings, "/actions/main/"),
            "/actions/Main"
        );
    }

    #[test]
    fn action_set_lookup_prefers_an_exact_key() {
        let mut bindings = BTreeMap::new();
        bindings.insert(
            "/actions/main".to_owned(),
            ActionSetBindings::default(),
        );
        bindings.insert(
            "/actions/Main".to_owned(),
            ActionSetBindings::default(),
        );

        assert_eq!(
            resolved_action_set_name(&bindings, "/actions/Main"),
            "/actions/Main"
        );
    }
}
