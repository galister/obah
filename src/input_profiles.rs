use crate::types::profile::InputProfile;

pub const INPUT_PROFILES_BYTES: &[(&str, &[u8])] = &[
    (
        "gamepad",
        include_bytes!("../profiles/gamepad_profile.json"),
    ),
    (
        "knuckles",
        include_bytes!("../profiles/index_controller_profile.json"),
    ),
    (
        "oculus_touch",
        include_bytes!("../profiles/touch_profile.json"),
    ),
    ("rift", include_bytes!("../profiles/rift_profile.json")),
    (
        "svl_hand_interaction_augmented",
        include_bytes!("../profiles/svl_hand_interaction_augmented_input_profile.json"),
    ),
    (
        "vive_controller",
        include_bytes!("../profiles/vive_controller_profile.json"),
    ),
    (
        "vive_focus3_controller",
        include_bytes!("../profiles/vive_focus3_controller_profile.json"),
    ),
];

pub fn load_input_profile(name: &str) -> Option<InputProfile> {
    INPUT_PROFILES_BYTES
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, bytes)| serde_json::from_slice(bytes).ok())
}
