use ratatui::widgets::ListState;

pub fn apply_selection(list_state: &mut ListState, index: usize, total_items: usize) {
    if total_items > 0 {
        let clamped = index.min(total_items - 1);
        list_state.select(Some(clamped));
    } else {
        list_state.select(None);
    }
}
