use ris_data::gameloop::frame_data::FrameData;
use ris_data::gameloop::gameloop_state::GameloopState;
use ris_data::gameloop::input_data::InputData;
use ris_data::gameloop::logic_data::LogicData;
use ris_data::gameloop::output_data::OutputData;
use ris_math::matrix4x4::Matrix4x4;
use ris_video::video::Video;

pub struct OutputFrame {
    video: Video,
}

impl OutputFrame {
    pub fn new(video: Video) -> Result<Self, String> {
        Ok(Self {
            video,
        })
    }

    pub fn run(
        &mut self,
        _current: &mut OutputData,
        _previous: &OutputData,
        input: &InputData,
        logic: &LogicData,
        _frame: &FrameData,
    ) -> GameloopState {
        if input.window_size_changed.is_some() {
            self.video.on_window_resize();
        }

        match self.video.update() {
            Ok(()) => GameloopState::WantsToContinue,
            Err(e) => GameloopState::Error(e),
        }
    }
}
