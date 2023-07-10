use crate::input::{
    gamepad_data::GamepadData, general_data::GeneralData, keyboard_data::KeyboardData,
    mouse_data::MouseData,
};

#[derive(Default, Clone)]
pub struct InputData {
    pub mouse: MouseData,
    pub keyboard: KeyboardData,
    pub gamepad: GamepadData,
    pub general: GeneralData,

    pub window_size_changed: Option<(i32, i32)>,
}
