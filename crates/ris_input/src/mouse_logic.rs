use ris_data::input::mouse_data::MouseData;
use sdl2::event::Event;

pub fn pre_update_mouse(mouse_data: &mut MouseData) {
    mouse_data.xrel = 0;
    mouse_data.yrel = 0;
    mouse_data.wheel_xrel = 0;
    mouse_data.wheel_yrel = 0;
}

pub fn update_mouse(mouse_data: &mut MouseData, event: &Event) {
    if let Event::MouseMotion {
        x, y, xrel, yrel, ..
    } = event
    {
        mouse_data.x = *x;
        mouse_data.y = *y;
        mouse_data.xrel += xrel;
        mouse_data.yrel += yrel;
    }

    if let Event::MouseWheel { x, y, .. } = event {
        mouse_data.wheel_xrel += x;
        mouse_data.wheel_yrel += y;
    }
}

pub fn post_update_mouse(
    new_mouse_data: &mut MouseData,
    old_mouse_data: &MouseData,
    mouse_state: sdl2::mouse::MouseState,
) {
    let new_state = mouse_state.to_sdl_state();
    let old_state = old_mouse_data.buttons.hold();
    new_mouse_data.buttons.update(&new_state, &old_state);
}