use crate::memory::joypads::{JoyPad, JoyPadButtons};
use crate::engine::config::EmulatorConfig;

pub struct ControllerState {
    pub a: bool, pub b: bool,
    pub up: bool, pub down: bool,
    pub left: bool, pub right: bool,
    pub start: bool, pub select: bool,
}

pub fn apply_input(joypad: &mut JoyPad, state: &ControllerState, config: &EmulatorConfig) {
    joypad.set_button(JoyPadButtons::A, state.a);
    joypad.set_button(JoyPadButtons::B, state.b);
    joypad.set_button(JoyPadButtons::START, state.start);
    joypad.set_button(JoyPadButtons::SELECT, state.select);
    
    let (left, right, up, down) = if config.allow_opposite_directions {
        (state.left, state.right, state.up, state.down)
    } else {
        let horiz_conflict = state.left && state.right;
        let vert_conflict = state.up && state.down;

        (
            state.left && !horiz_conflict, 
            state.right && !horiz_conflict, 
            state.up && !vert_conflict, 
            state.down && !vert_conflict
        )
    };
    
    joypad.set_button(JoyPadButtons::LEFT,  left);
    joypad.set_button(JoyPadButtons::RIGHT, right);
    joypad.set_button(JoyPadButtons::UP,    up);
    joypad.set_button(JoyPadButtons::DOWN,  down);
}