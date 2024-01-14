use std::path::PathBuf;

use imgui::BackendFlags;
use imgui::ConfigFlags;
use imgui::Context;
use imgui::Io;
use imgui::Ui;
use sdl2::keyboard::Mod;
use sdl2::keyboard::Scancode;
use sdl2::mouse::Cursor;

use ris_data::gameloop::frame::Frame;
use ris_data::gameloop::logic_data::LogicData;
use ris_data::info::app_info::AppInfo;
use ris_data::input::buttons::Buttons;
use ris_data::input::keys::KEY_STATE_SIZE;
use ris_error::RisResult;

use crate::vulkan::renderer::Renderer;

pub struct ImguiBackend {
    // context
    context: Context,

    // platform:
    cursor_instance: Option<Cursor>,
}

impl ImguiBackend {
    pub fn context(&mut self) -> &mut Context {
        &mut self.context
    }

    pub fn init(app_info: &AppInfo) -> RisResult<Self> {
        // setup context
        let mut dir = PathBuf::from(&app_info.file.pref_path);
        dir.push("imgui");

        if !dir.exists() {
            ris_error::unroll!(
                std::fs::create_dir_all(&dir),
                "failed to create imgui dir: {:?}",
                &dir,
            )?;
        }

        let mut ini_filepath = PathBuf::from(&dir);
        ini_filepath.push("imgui.ini");

        let mut log_filepath = PathBuf::from(&dir);
        log_filepath.push("imgui_log.txt");

        let mut context = Context::create();
        context.set_ini_filename(Some(ini_filepath));
        context.set_log_filename(Some(log_filepath));

        let font_atlas = context.fonts();
        font_atlas.add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

        // setup platform
        let io = context.io_mut();

        io.backend_flags.insert(BackendFlags::HAS_MOUSE_CURSORS);
        io.backend_flags.insert(BackendFlags::HAS_SET_MOUSE_POS);

        context.set_platform_name(Some(String::from("ris_engine sdl2 backend")));

        // setup renderer
        // todo

        Ok(Self {
            context,
            cursor_instance: None,
        })
    }

    pub fn prepare_frame(
        &mut self,
        logic_data: &LogicData,
        frame: Frame,
        renderer: &Renderer,
    ) -> &mut Ui {
        let mouse_cursor = self.context.mouse_cursor();
        let io = self.context.io_mut();

        io.update_delta_time(frame.previous_duration());

        // mouse input
        let mouse = &logic_data.mouse;

        let x = mouse.wheel_xrel;
        let y = mouse.wheel_yrel;
        if x != 0 || y != 0 {
            io.add_mouse_wheel_event([x as f32, y as f32]);
        }

        let buttons = &mouse.buttons;
        forward_mouse_button_event(io, buttons, 0);
        forward_mouse_button_event(io, buttons, 1);
        forward_mouse_button_event(io, buttons, 2);
        forward_mouse_button_event(io, buttons, 3);
        forward_mouse_button_event(io, buttons, 4);

        // keyboard input
        let keyboard = &logic_data.keyboard;

        let mod_state = keyboard.mod_state;

        io.add_key_event(
            imgui::Key::ModShift,
            mod_state.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        );
        io.add_key_event(
            imgui::Key::ModCtrl,
            mod_state.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        );
        io.add_key_event(
            imgui::Key::ModAlt,
            mod_state.intersects(Mod::LALTMOD | Mod::RALTMOD),
        );
        io.add_key_event(
            imgui::Key::ModSuper,
            mod_state.intersects(Mod::LGUIMOD | Mod::RGUIMOD),
        );

        let keys_down = keyboard.keys.down();
        let keys_up = keyboard.keys.up();
        for i in 0..KEY_STATE_SIZE {
            let down = keys_down[i];
            let up = keys_up[i];

            let event = if down {
                Some(true)
            } else if up {
                Some(false)
            } else {
                None
            };

            if let Some(pressed) = event {
                if let Some(scancode) = Scancode::from_i32(i as i32) {
                    forward_keyboard_key_event(io, scancode, pressed);
                }
            }
        }

        // text input
        for text in &keyboard.text_input {
            text.chars().for_each(|c| io.add_input_character(c));
        }

        // prepare frame
        let window_size = renderer.window.size();
        let window_drawable_size = renderer.window.vulkan_drawable_size();

        io.display_size = [(window_size.0 as f32), (window_size.1 as f32)];
        io.display_framebuffer_scale = [
            (window_drawable_size.0 as f32) / (window_size.0 as f32),
            (window_drawable_size.1 as f32) / (window_size.1 as f32),
        ];

        // update mouse
        if io.want_set_mouse_pos {
            ris_log::warning!("set mouse pos not implemented!");
        }

        io.mouse_pos = [mouse.x as f32, mouse.y as f32];

        if !io
            .config_flags
            .contains(ConfigFlags::NO_MOUSE_CURSOR_CHANGE)
        {
            //ris_log::warning!("set mouse cursor is not implemented!");
        }

        self.context.new_frame()
    }
}

fn forward_mouse_button_event(io: &mut Io, buttons: &Buttons, button: usize) {
    debug_assert!(button < 5);

    let event = if buttons.is_down(1 << button) {
        Some(true)
    } else if buttons.is_up(1 << button) {
        Some(false)
    } else {
        None
    };

    if let Some(pressed) = event {
        let mouse_button = match button {
            0 => imgui::MouseButton::Left,
            1 => imgui::MouseButton::Right,
            2 => imgui::MouseButton::Middle,
            3 => imgui::MouseButton::Extra1,
            4 => imgui::MouseButton::Extra2,
            _ => return,
        };

        io.add_mouse_button_event(mouse_button, pressed);
    }
}

fn forward_keyboard_key_event(io: &mut Io, scancode: Scancode, pressed: bool) {
    let imgui_scancode = match scancode {
        Scancode::A => imgui::Key::A,
        Scancode::B => imgui::Key::B,
        Scancode::C => imgui::Key::C,
        Scancode::D => imgui::Key::D,
        Scancode::E => imgui::Key::E,
        Scancode::F => imgui::Key::F,
        Scancode::G => imgui::Key::G,
        Scancode::H => imgui::Key::H,
        Scancode::I => imgui::Key::I,
        Scancode::J => imgui::Key::J,
        Scancode::K => imgui::Key::K,
        Scancode::L => imgui::Key::L,
        Scancode::M => imgui::Key::M,
        Scancode::N => imgui::Key::N,
        Scancode::O => imgui::Key::O,
        Scancode::P => imgui::Key::P,
        Scancode::Q => imgui::Key::Q,
        Scancode::R => imgui::Key::R,
        Scancode::S => imgui::Key::S,
        Scancode::T => imgui::Key::T,
        Scancode::U => imgui::Key::U,
        Scancode::V => imgui::Key::V,
        Scancode::W => imgui::Key::W,
        Scancode::X => imgui::Key::X,
        Scancode::Y => imgui::Key::Y,
        Scancode::Z => imgui::Key::Z,
        Scancode::Num1 => imgui::Key::Keypad1,
        Scancode::Num2 => imgui::Key::Keypad2,
        Scancode::Num3 => imgui::Key::Keypad3,
        Scancode::Num4 => imgui::Key::Keypad4,
        Scancode::Num5 => imgui::Key::Keypad5,
        Scancode::Num6 => imgui::Key::Keypad6,
        Scancode::Num7 => imgui::Key::Keypad7,
        Scancode::Num8 => imgui::Key::Keypad8,
        Scancode::Num9 => imgui::Key::Keypad9,
        Scancode::Num0 => imgui::Key::Keypad0,
        Scancode::Return => imgui::Key::Enter,
        Scancode::Escape => imgui::Key::Escape,
        Scancode::Backspace => imgui::Key::Backspace,
        Scancode::Tab => imgui::Key::Tab,
        Scancode::Space => imgui::Key::Space,
        Scancode::Minus => imgui::Key::Minus,
        Scancode::Equals => imgui::Key::Equal,
        Scancode::LeftBracket => imgui::Key::LeftBracket,
        Scancode::RightBracket => imgui::Key::RightBracket,
        Scancode::Backslash => imgui::Key::Backslash,
        Scancode::Semicolon => imgui::Key::Semicolon,
        Scancode::Apostrophe => imgui::Key::Apostrophe,
        Scancode::Grave => imgui::Key::GraveAccent,
        Scancode::Comma => imgui::Key::Comma,
        Scancode::Period => imgui::Key::Period,
        Scancode::Slash => imgui::Key::Slash,
        Scancode::CapsLock => imgui::Key::CapsLock,
        Scancode::F1 => imgui::Key::F1,
        Scancode::F2 => imgui::Key::F2,
        Scancode::F3 => imgui::Key::F3,
        Scancode::F4 => imgui::Key::F4,
        Scancode::F5 => imgui::Key::F5,
        Scancode::F6 => imgui::Key::F6,
        Scancode::F7 => imgui::Key::F7,
        Scancode::F8 => imgui::Key::F8,
        Scancode::F9 => imgui::Key::F9,
        Scancode::F10 => imgui::Key::F10,
        Scancode::F11 => imgui::Key::F11,
        Scancode::F12 => imgui::Key::F12,
        Scancode::PrintScreen => imgui::Key::PrintScreen,
        Scancode::ScrollLock => imgui::Key::ScrollLock,
        Scancode::Pause => imgui::Key::Pause,
        Scancode::Insert => imgui::Key::Insert,
        Scancode::Home => imgui::Key::Home,
        Scancode::PageUp => imgui::Key::PageUp,
        Scancode::Delete => imgui::Key::Delete,
        Scancode::End => imgui::Key::End,
        Scancode::PageDown => imgui::Key::PageDown,
        Scancode::Right => imgui::Key::RightArrow,
        Scancode::Left => imgui::Key::LeftArrow,
        Scancode::Down => imgui::Key::DownArrow,
        Scancode::Up => imgui::Key::UpArrow,
        Scancode::KpDivide => imgui::Key::KeypadDivide,
        Scancode::KpMultiply => imgui::Key::KeypadMultiply,
        Scancode::KpMinus => imgui::Key::KeypadSubtract,
        Scancode::KpPlus => imgui::Key::KeypadAdd,
        Scancode::KpEnter => imgui::Key::KeypadEnter,
        Scancode::Kp1 => imgui::Key::Keypad1,
        Scancode::Kp2 => imgui::Key::Keypad2,
        Scancode::Kp3 => imgui::Key::Keypad3,
        Scancode::Kp4 => imgui::Key::Keypad4,
        Scancode::Kp5 => imgui::Key::Keypad5,
        Scancode::Kp6 => imgui::Key::Keypad6,
        Scancode::Kp7 => imgui::Key::Keypad7,
        Scancode::Kp8 => imgui::Key::Keypad8,
        Scancode::Kp9 => imgui::Key::Keypad9,
        Scancode::Kp0 => imgui::Key::Keypad0,
        Scancode::KpPeriod => imgui::Key::KeypadDecimal,
        Scancode::Application => imgui::Key::Menu,
        Scancode::KpEquals => imgui::Key::KeypadEqual,
        Scancode::Menu => imgui::Key::Menu,
        Scancode::LCtrl => imgui::Key::LeftCtrl,
        Scancode::LShift => imgui::Key::LeftShift,
        Scancode::LAlt => imgui::Key::LeftAlt,
        Scancode::LGui => imgui::Key::LeftSuper,
        Scancode::RCtrl => imgui::Key::RightCtrl,
        Scancode::RShift => imgui::Key::RightShift,
        Scancode::RAlt => imgui::Key::RightAlt,
        Scancode::RGui => imgui::Key::RightSuper,
        _ => return,
    };

    io.add_key_event(imgui_scancode, pressed);
}
