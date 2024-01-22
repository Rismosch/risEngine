use std::sync::Arc;
use std::time::Instant;

use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::KeyboardUtil;
use sdl2::keyboard::Scancode;
use sdl2::EventPump;
use sdl2::GameControllerSubsystem;

use ris_data::gameloop::frame::Frame;
use ris_data::gameloop::gameloop_state::GameloopState;
use ris_data::god_state;
use ris_data::god_state::GodState;
use ris_data::input::action;
use ris_error::RisResult;
use ris_input::gamepad_logic::GamepadLogic;
use ris_input::general_logic::update_general;
use ris_input::keyboard_logic;
use ris_input::mouse_logic;
use ris_jobs::job_future::JobFuture;
use ris_math::quaternion::Quaternion;
use ris_math::vector3;
use ris_math::vector3::Vector3;

const CRASH_TIMEOUT_IN_SECS: u64 = 5;

#[cfg(debug_assertions)]
fn reload_shaders() -> JobFuture<()> {
    use ris_asset::asset_importer;

    ris_jobs::job_system::submit(|| {
        let result = asset_importer::import_all(
            asset_importer::DEFAULT_SOURCE_DIRECTORY,
            asset_importer::DEFAULT_TARGET_DIRECTORY,
        );

        if let Err(error) = result {
            ris_log::error!("failed to import shaders: {}", error);
        }
    })
}

#[cfg(not(debug_assertions))]
fn reload_shaders() -> JobFuture<()> {
    ris_log::warning!("shaders can only be reloaded in a debug build!");
    JobFuture::done()
}

pub struct LogicFrame {
    // input
    event_pump: EventPump,
    keyboard_util: KeyboardUtil,

    gamepad_logic: GamepadLogic,

    // general
    restart_timestamp: Instant,
    crash_timestamp: Instant,
}

impl LogicFrame {
    pub fn new(
        event_pump: EventPump,
        keyboard_util: KeyboardUtil,
        controller_subsystem: GameControllerSubsystem,
    ) -> Self {
        Self {
            event_pump,
            keyboard_util,
            gamepad_logic: GamepadLogic::new(controller_subsystem),

            crash_timestamp: Instant::now(),
            restart_timestamp: Instant::now(),
        }
    }

    pub fn run(
        &mut self,
        frame: Frame,
        state: Arc<GodState>,
    ) -> RisResult<GameloopState> {
        // input
        mouse_logic::pre_events(&mut state.front_mut().input.mouse);
        keyboard_logic::pre_events(&mut state.front_mut().input.keyboard);

        for event in self.event_pump.poll_iter() {
            if let Event::Quit { .. } = event {
                return Ok(GameloopState::WantsToQuit);
            };

            if let Event::Window {
                win_event: WindowEvent::SizeChanged(w, h),
                ..
            } = event
            {
                state.front_mut().window_event = god_state::WindowEvent::SizeChanged(w, h);
                ris_log::trace!("window changed size to {}x{}", w, h);
            }

            mouse_logic::handle_event(&mut state.front_mut().input.mouse, &event);
            keyboard_logic::handle_event(&mut state.front_mut().input.keyboard, &event);
            self.gamepad_logic.handle_event(&event);
        }

        mouse_logic::post_events(
            &mut state.front_mut().input.mouse,
            self.event_pump.mouse_state(),
        );

        keyboard_logic::post_events(
            &mut state.front_mut().input.keyboard,
            self.event_pump.keyboard_state(),
            self.keyboard_util.mod_state(),
        );

        self.gamepad_logic.post_events(&mut state.front_mut().input.gamepad);

        update_general(state.clone());

        // manual restart
        if state.front().input.keyboard.keys.is_hold(Scancode::F1) {
            let duration = Instant::now() - self.restart_timestamp;
            let seconds = duration.as_secs();

            if seconds >= CRASH_TIMEOUT_IN_SECS {
                ris_log::fatal!("manual restart reqeusted");
                return Ok(GameloopState::WantsToRestart);
            }
        } else {
            self.restart_timestamp = Instant::now();
        }

        // manual crash
        if state.front().input.keyboard.keys.is_hold(Scancode::F4) {
            let duration = Instant::now() - self.crash_timestamp;
            let seconds = duration.as_secs();

            if seconds >= CRASH_TIMEOUT_IN_SECS {
                ris_log::fatal!("manual crash requested");
                return ris_error::new_result!("manual crash");
            }
        } else {
            self.crash_timestamp = Instant::now();
        }

        // reload shaders
        let mut import_shader_future = None;
        if state.front().input.keyboard.keys.is_down(Scancode::F6) {
            state.front_mut().reload_shaders = true;
            let future = reload_shaders();
            import_shader_future = Some(future);
        }

        // game logic
        let rotation_speed = 2. * frame.average_seconds();
        let movement_speed = 2. * frame.average_seconds();
        let mouse_speed = 20. * frame.average_seconds();

        if state.front().input.mouse.buttons.is_hold(action::OK) {
            let yrel = mouse_speed * state.front().input.mouse.yrel as f32;
            let xrel = mouse_speed * state.front().input.mouse.xrel as f32;
            state.front_mut().camera_vertical_angle -= yrel;
            state.front_mut().camera_horizontal_angle -= xrel;
        } else if state.front().input.general.buttons.is_down(action::OK) {
            state.front_mut().camera_horizontal_angle = 0.0;
            state.front_mut().camera_vertical_angle = 0.0;
            state.front_mut().camera_position = Vector3::new(0., -1., 0.);
        }

        if state.front().input.general.buttons.is_hold(action::CAMERA_UP) {
            state.front_mut().camera_vertical_angle += rotation_speed;
        }

        if state.front().input.general.buttons.is_hold(action::CAMERA_DOWN) {
            state.front_mut().camera_vertical_angle -= rotation_speed;
        }

        if state.front().input.general.buttons.is_hold(action::CAMERA_LEFT) {
            state.front_mut().camera_horizontal_angle += rotation_speed;
        }

        if state.front().input.general.buttons.is_hold(action::CAMERA_RIGHT) {
            state.front_mut().camera_horizontal_angle -= rotation_speed;
        }
        
        let mut camera_horizontal_angle = state.front().camera_horizontal_angle;
        let mut camera_vertical_angle = state.front().camera_vertical_angle;
        while camera_horizontal_angle < 0. {
            camera_horizontal_angle += ris_math::PI_2;
        }
        while camera_horizontal_angle > ris_math::PI_2 {
            camera_horizontal_angle -= ris_math::PI_2;
        }
        camera_vertical_angle = ris_math::clamp(
            camera_vertical_angle,
            -ris_math::PI_0_5,
            ris_math::PI_0_5,
        );
        state.front_mut().camera_horizontal_angle = camera_horizontal_angle;
        state.front_mut().camera_vertical_angle = camera_vertical_angle;

        let rotation1 = Quaternion::from_angle_axis(state.front().camera_vertical_angle, vector3::RIGHT);
        let rotation2 = Quaternion::from_angle_axis(state.front().camera_horizontal_angle, vector3::UP);
        state.front_mut().camera_rotation = rotation2 * rotation1;

        if state.front().input.general.buttons.is_hold(action::MOVE_UP) {
            let forward = state.front().camera_rotation.rotate(vector3::FORWARD);
            state.front_mut().camera_position += movement_speed * forward;
        }

        if state.front().input.general.buttons.is_hold(action::MOVE_DOWN) {
            let forward = state.front().camera_rotation.rotate(vector3::FORWARD);
            state.front_mut().camera_position -= movement_speed * forward;
        }

        if state.front().input.general.buttons.is_hold(action::MOVE_LEFT) {
            let right = state.front().camera_rotation.rotate(vector3::RIGHT);
            state.front_mut().camera_position -= movement_speed * right;
        }

        if state.front().input.general.buttons.is_hold(action::MOVE_RIGHT) {
            let right = state.front().camera_rotation.rotate(vector3::RIGHT);
            state.front_mut().camera_position += movement_speed * right;
        }

        let workers = state.front().settings.job().get_workers();
        if let Some(workers) = workers {
            if state.front().input.keyboard.keys.is_hold(Scancode::LCtrl) {
                if state.front().input.keyboard.keys.is_down(Scancode::Up) {
                    let new_workers = Some(workers.saturating_add(1));
                    state.front_mut().settings.job_mut().set_workers(new_workers);
                }
                if state.front().input.keyboard.keys.is_down(Scancode::Down) {
                    let new_workers = Some(workers.saturating_sub(1));
                    state.front_mut().settings.job_mut().set_workers(new_workers);
                }
                if state.front().input.keyboard.keys.is_down(Scancode::Return) {
                    state.front_mut().settings.request_save();
                }
            }
        }

        if state.front().input.keyboard.keys.is_down(Scancode::F) {
            ris_log::debug!(
                "{:?} ({} fps)",
                frame.average_duration(),
                frame.average_fps()
            );
        }

        if let Some(future) = import_shader_future {
            future.wait();
        }

        Ok(GameloopState::WantsToContinue)
    }
}
