pub mod controller_type;
pub mod edit_bindings;
pub mod game_select;
pub mod list_utils;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    GameSelect,
    ControllerType,
    EditBindings,
}

impl std::fmt::Display for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Screen::GameSelect => write!(f, "Game Select"),
            Screen::ControllerType => write!(f, "Controller Type"),
            Screen::EditBindings => write!(f, "Edit Bindings"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingSourceSelection {
    Default,
    Xrizer,
    Vapor,
    OpenComposite,
    Scratch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenAction {
    None,
    GoBack,
    Quit,
    ControllerSelected {
        controller_type: String,
        binding_source: BindingSourceSelection,
    },
}
