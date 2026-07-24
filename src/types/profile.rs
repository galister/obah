use crate::types::manifest::{BindingUiMode, ProfileDefaultBinding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InputProfile {
    pub jsonid: String,
    pub controller_type: String,
    #[serde(default)]
    pub legacy_binding: Option<String>,
    #[serde(default)]
    pub legacy_profile: Option<String>,
    #[serde(default)]
    pub device_class: Option<String>,
    #[serde(default)]
    pub resource_root: Option<String>,
    #[serde(default)]
    pub driver_name: Option<String>,
    #[serde(default)]
    pub hand_priority: Option<i32>,
    pub input_bindingui_mode: BindingUiMode,
    #[serde(default)]
    pub should_show_binding_errors: Option<bool>,
    #[serde(default)]
    pub remapping: Option<String>,
    #[serde(default)]
    pub input_bindingui_left: Option<crate::types::manifest::BindingUi>,
    #[serde(default)]
    pub input_bindingui_right: Option<crate::types::manifest::BindingUi>,
    pub input_source: HashMap<String, InputSourceValue>,
    #[serde(default)]
    pub simulation_settings: Option<SimulationSettings>,
    #[serde(default)]
    pub mirror_mappings: Option<Vec<Vec<String>>>,
    #[serde(default)]
    pub default_bindings: Option<Vec<ProfileDefaultBinding>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum InputSourceValue {
    Def(InputSourceDef),
    MapInputSource(HashMap<String, InputSourceDef>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InputSourceDef {
    pub r#type: InputSourceType,
    #[serde(default)]
    pub binding_image_point: Option<[i32; 2]>,
    #[serde(default)]
    pub click: bool,
    #[serde(default)]
    pub value: bool,
    #[serde(default)]
    pub touch: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub skeleton: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub input_activity_path: Option<String>,
    #[serde(default)]
    pub input_activity_threshold: Option<f64>,
    #[serde(default)]
    pub value_source: Option<String>,
    #[serde(default)]
    pub capsense_source: Option<String>,
    #[serde(default)]
    pub force_source: Option<String>,
    #[serde(default)]
    pub input_source: Option<HashMap<String, InputSourceDef>>,
}

impl InputSourceDef {
    pub fn supports_toggle_button(&self) -> bool {
        match &self.r#type {
            // These source types imply a usable digital or scalar activation,
            // even when an older profile omits the individual capability flags.
            InputSourceType::Button | InputSourceType::Trigger | InputSourceType::Pinch => true,

            // A two-axis source needs a component that can act as the toggle
            // activation. Position alone is not sufficient.
            InputSourceType::Joystick | InputSourceType::Trackpad => {
                self.click || self.touch || self.value || self.force
            }

            InputSourceType::Pose | InputSourceType::Skeleton | InputSourceType::Vibration => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InputSourceType {
    Button,
    Trigger,
    Joystick,
    Trackpad,
    Pose,
    Skeleton,
    Vibration,
    Pinch,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulationSettings {
    #[serde(default)]
    pub hmd_profile: Option<String>,
    #[serde(default)]
    pub modelnumber: Option<String>,
    #[serde(default)]
    pub left_modelnumber: Option<String>,
    #[serde(default)]
    pub right_modelnumber: Option<String>,
    #[serde(default)]
    pub serialnumber: Option<String>,
    #[serde(default)]
    pub left_serialnumber: Option<String>,
    #[serde(default)]
    pub right_serialnumber: Option<String>,
    #[serde(default)]
    pub rendermodel: Option<String>,
    #[serde(default)]
    pub left_rendermodel: Option<String>,
    #[serde(default)]
    pub right_rendermodel: Option<String>,
    #[serde(default)]
    pub registered_device_type: Option<String>,
    #[serde(default)]
    pub left_registered_device_type: Option<String>,
    #[serde(default)]
    pub right_registered_device_type: Option<String>,
    #[serde(default)]
    pub tracking_system_name: Option<String>,
    #[serde(default)]
    pub manufacturer_name: Option<String>,
    #[serde(default)]
    pub legacy_buttons: Option<Vec<u32>>,
    #[serde(default)]
    pub legacy_axis: Option<Vec<u32>>,
}
