use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

use ris_jobs::job_system;
use ris_math::quaternion::Quaternion;
use ris_math::vector3::Vector3;

use crate::settings::Settings;

#[derive(Clone, Copy)]
pub enum WindowEvent {
    None,
    SizeChanged(i32, i32),
}

impl Default for WindowEvent {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Default)]
pub struct GodStateData {
    // events
    pub reload_shaders: bool,
    pub window_event: WindowEvent,

    // general
    pub camera_horizontal_angle: f32,
    pub camera_vertical_angle: f32,
    pub camera_position: Vector3,
    pub camera_rotation: Quaternion,

    // settings
    pub settings: Settings,
}

pub type GodStateLock = RwLock<GodStateData>;

pub struct GodState {
    front: GodStateLock,
    back: GodStateLock,
}

impl GodStateData {
    pub fn new(settings: Settings) -> GodStateLock {
        let data = GodStateData { settings, ..Default::default()};

        RwLock::new(data)
    }
}

impl GodState {
    pub fn new(front: GodStateLock, back: GodStateLock) -> Arc<Self> {
        let double_buffer = GodState { front, back };

        Arc::new(double_buffer)
    }

    pub fn front(&self) -> RwLockReadGuard<GodStateData> {
        job_system::lock_read(&self.front)
    }

    pub fn front_mut(&self) -> RwLockWriteGuard<GodStateData>{
        job_system::lock_write(&self.front)
    }

    pub fn back(&self) -> RwLockReadGuard<GodStateData>{
        job_system::lock_read(&self.back)
    }

    pub fn back_mut(&self) -> RwLockWriteGuard<GodStateData> {
        job_system::lock_write(&self.back)
    }

    pub fn copy_front_to_back(&self) {
        let mut front = self.front_mut();
        let mut back = self.back_mut();

        // events
        back.reload_shaders = front.reload_shaders;
        front.reload_shaders = false;
        back.window_event = front.window_event;
        front.window_event = WindowEvent::None;

        // general
        back.camera_horizontal_angle = front.camera_horizontal_angle;
        back.camera_vertical_angle = front.camera_vertical_angle;
        back.camera_position = front.camera_position;
        back.camera_rotation = front.camera_rotation;

        // settings
        back.settings = front.settings.clone();
        front.settings.reset();
    }
}
