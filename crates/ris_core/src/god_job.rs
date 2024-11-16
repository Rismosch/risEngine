use ris_data::ecs::components::mesh_renderer::MeshRendererComponent;
use ris_data::ecs::decl::GameObjectHandle;
use ris_data::ecs::decl::MeshRendererComponentHandle;
use ris_data::ecs::decl::VideoMeshHandle;
use ris_data::ecs::game_object::GetFrom;
use ris_data::ecs::id::GameObjectKind;
use ris_data::ecs::mesh::Mesh;
use ris_data::gameloop::gameloop_state::GameloopState;
use ris_error::RisResult;
use ris_math::color::Rgb;
use ris_math::vector::Vec3;
use ris_jobs::job_system;

use crate::god_object::GodObject;

pub enum WantsTo {
    Quit,
    Restart,
}

pub fn run(mut god_object: GodObject) -> RisResult<WantsTo> {
    let mut frame_calculator = god_object.frame_calculator;

    // TESTING
    
    let mut rng = ris_rng::rng::Rng::new(ris_rng::rng::Seed::new()?);

    let count = 1;
    for i in 0..count {
        let game_object = GameObjectHandle::new(&god_object.state.scene, GameObjectKind::Movable)?;
        game_object.set_name(&god_object.state.scene, format!("mesh {}", i))?;
        let scale = count as f32;
        let position = rng.next_pos_3() * scale;
        let rotation = rng.next_rot();
        //game_object.set_local_position(&god_object.state.scene, position)?;
        //game_object.set_local_rotation(&god_object.state.scene, rotation)?;

        let physical_device_memory_properties = unsafe {
            god_object.output_frame.core.instance.get_physical_device_memory_properties(god_object.output_frame.core.suitable_device.physical_device)
        };

        let mesh = Mesh::primitive_cube();
        let video_mesh = VideoMeshHandle::new(&god_object.state.scene)?;
        video_mesh.upload(
            &god_object.state.scene,
            &god_object.output_frame.core.device,
            physical_device_memory_properties,
            mesh,
        )?;
        let mesh_renderer: MeshRendererComponentHandle = game_object.add_component(&god_object.state.scene)?.into();
        mesh_renderer.set_video_mesh(&god_object.state.scene, video_mesh)?;
    }

    // TESTING END

    loop {
        ris_debug::profiler::new_frame()?;
        let frame = frame_calculator.bump_and_create_frame();

        // reset events
        let mut r = ris_debug::new_record!("main loop");

        let previous_state = god_object.state.clone();
        god_object.state.reset_events();

        // game loop
        ris_debug::add_record!(r, "submit save settings future")?;
        let save_settings_future = job_system::submit(move || {
            let settings_serializer = god_object.settings_serializer;
            let state = previous_state;

            let settings = &state.settings;

            let result = if settings.save_requested() {
                settings_serializer.serialize(settings)
            } else {
                Ok(())
            };

            (settings_serializer, result)
        });

        ris_debug::add_record!(r, "logic frame")?;
        let logic_result = god_object.logic_frame.run(frame, &mut god_object.state);

        for script in god_object.state.scene.script_components.iter() {
            let mut aref_mut = script.borrow_mut();
            if aref_mut.is_alive {
                aref_mut.update(frame, &god_object.state)?;
            }
        }

        ris_debug::add_record!(r, "output frame")?;
        let output_result =
            god_object
                .output_frame
                .run(frame, &mut god_object.state, &god_object.god_asset);

        // wait for jobs
        ris_debug::add_record!(r, "wait for jobs")?;
        let (new_settings_serializer, save_settings_result) = save_settings_future.wait(None)?;

        // update buffers
        ris_debug::add_record!(r, "update buffers")?;
        god_object.settings_serializer = new_settings_serializer;

        // restart job system
        ris_debug::add_record!(r, "restart job system")?;

        let settings = &god_object.state.settings;
        if settings.job().changed() {
            ris_log::debug!("job workers changed. restarting job system...");
            drop(god_object.job_system_guard);

            let cpu_count = god_object.app_info.cpu.cpu_count;
            let workers = crate::determine_thread_count(&god_object.app_info, settings);

            let new_guard = job_system::init(
                job_system::DEFAULT_BUFFER_CAPACITY,
                cpu_count,
                workers,
                true,
            );
            god_object.job_system_guard = new_guard;

            ris_log::debug!("job system restarted!");
        }

        // handle errors
        ris_debug::add_record!(r, "handle errors")?;

        save_settings_result?;
        let logic_state = logic_result?;
        let output_state = output_result?;

        ris_debug::end_record!(r)?;

        // continue?
        let wants_to_quit =
            logic_state == GameloopState::WantsToQuit || output_state == GameloopState::WantsToQuit;
        let wants_to_restart = logic_state == GameloopState::WantsToRestart
            || output_state == GameloopState::WantsToRestart;

        let wants_to_option = if wants_to_quit {
            Some(WantsTo::Quit)
        } else if wants_to_restart {
            Some(WantsTo::Restart)
        } else {
            None
        };

        let Some(wants_to) = wants_to_option else {
            continue;
        };

        // shutdown
        for script in god_object.state.scene.script_components.iter() {
            let mut aref_mut = script.borrow_mut();
            if aref_mut.is_alive {
                aref_mut.end(&god_object.state.scene)?;
            }
        }

        god_object.output_frame.wait_idle()?;
        god_object.state.scene.free(&god_object.output_frame.core.device);

        return Ok(wants_to);
    }
}
