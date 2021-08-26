use crate::{
    gltf_loader::load_models_from_gltf,
    resources::{RenderContext, XrContext},
    schedule_functions::{begin_frame, end_frame},
    systems::{
        animation_system, hands_system, rendering_system, skinning_system,
        update_parent_transform_matrix_system, update_transform_matrix_system,
    },
    HothamResult, Program,
};
use anyhow::Result;

use legion::{Resources, Schedule, World};
use openxr as xr;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use xr::SessionState;

#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_INPUT: u32 = 1;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

pub struct App<P: Program> {
    _program: P,
    should_quit: Arc<AtomicBool>,
    world: World,
    resources: Resources,
    schedule: Schedule,
    _term: console::Term,
    _debug_messages: Vec<(String, String)>,
    #[allow(dead_code)]
    resumed: bool,
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(mut program: P) -> HothamResult<Self> {
        let (xr_context, vulkan_context) = XrContext::new()?;
        let render_context = RenderContext::new(&vulkan_context, &xr_context)?;
        println!("[HOTHAM_INIT] Loading models..");
        let gltf_data = program.get_gltf_data();
        let models = load_models_from_gltf(
            gltf_data,
            &vulkan_context,
            render_context.descriptor_set_layouts.mesh_layout,
        )?;
        let mut resources = Resources::default();
        resources.insert(xr_context);
        resources.insert(vulkan_context);
        resources.insert(render_context);
        resources.insert(0 as usize);
        let world = program.init(models, &mut resources)?;
        println!("[HOTHAM_INIT] ..done!");

        println!("[HOTHAM_INIT] Creating schedule..");
        let schedule = Schedule::builder()
            .add_thread_local_fn(begin_frame)
            .add_system(hands_system())
            .add_system(animation_system())
            .add_system(update_transform_matrix_system())
            .add_system(update_parent_transform_matrix_system())
            .add_system(skinning_system())
            .add_system(rendering_system())
            .add_thread_local_fn(end_frame)
            .build();
        println!("[HOTHAM_INIT] DONE! INIT COMPLETE!");

        Ok(Self {
            _program: program,
            should_quit: Arc::new(AtomicBool::from(false)),
            resumed: true,
            _term: console::Term::buffered_stdout(),
            world,
            resources,
            _debug_messages: Default::default(),
            schedule,
        })
    }

    pub fn run(&mut self) -> HothamResult<()> {
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = self.should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))
                .map_err(anyhow::Error::new)?;
        }

        let mut event_buffer = Default::default();

        while !self.should_quit.load(Ordering::Relaxed) {
            #[cfg(target_os = "android")]
            self.process_android_events();
            let mut xr_context = self.resources.get_mut::<XrContext>().unwrap();
            let current_state = xr_context.poll_xr_event(&mut event_buffer)?;

            if current_state == SessionState::IDLE {
                sleep(Duration::from_secs(1));
                continue;
            }

            if current_state == SessionState::EXITING {
                break;
            }

            drop(xr_context);

            self.schedule.execute(&mut self.world, &mut self.resources);
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn show_debug_info(&self) -> Result<()> {
        // let frame_index = self.renderer.frame_index;
        // if self.renderer.frame_index % 72 != 0 {
        //     return Ok(());
        // };

        self._term.clear_screen()?;
        self._term.write_line("[APP_DEBUG]")?;
        // self.term.write_line(&format!("[Frame]: {}", frame_index))?;

        for (tag, message) in &self._debug_messages {
            self._term.write_line(&format!("[{}]: {}", tag, message))?;
        }
        self._term.flush()?;

        Ok(())
    }

    // fn update_hands(&mut self, predicted_display_time: xr::Time) -> Result<()> {
    //     let left_hand_pose = self
    //         .left_hand_space
    //         .locate(&self.reference_space, predicted_display_time)?
    //         .pose;
    //     let left_hand_grabbed = xr::ActionInput::get(
    //         &self.grab_action,
    //         &self.xr_session,
    //         self.left_hand_subaction_path,
    //     )?
    //     .current_state;
    // let position = mint::Vector3::from(left_hand_pose.position).into();
    // let orientation = mint::Quaternion::from(left_hand_pose.orientation).into();
    // self.left_hand.update_position(position, orientation);
    // self.left_hand
    //     .grip(left_hand_grabbed, &self.renderer.vulkan_context)?;

    // {
    //     let tag = "HANDS".to_string();
    //     let message = format!("Incoming orientation: {:?}", to_euler_degrees(orientation));
    //     self.debug_messages.push((tag.clone(), message));

    //     let message = format!(
    //         "Offset orientation: {:?}",
    //         to_euler_degrees(self.left_hand.grip_offset.1)
    //     );
    //     self.debug_messages.push((tag.clone(), message));

    //     let updated_orientation = (*self.left_hand.root_bone_node()).rotation;
    //     let message = format!(
    //         "Updated orientation: {:?}",
    //         to_euler_degrees(updated_orientation)
    //     );
    //     self.debug_messages.push((tag, message));
    // }

    //     let right_hand_pose = self
    //         .right_hand_space
    //         .locate(&self.reference_space, predicted_display_time)?
    //         .pose;
    //     let right_hand_grabbed = xr::ActionInput::get(
    //         &self.grab_action,
    //         &self.xr_session,
    //         self.right_hand_subaction_path,
    //     )?
    //     .current_state;
    //     // let position = mint::Vector3::from(right_hand_pose.position).into();
    //     // let orientation = mint::Quaternion::from(right_hand_pose.orientation).into();
    //     // self.right_hand.update_position(position, orientation);
    //     // self.right_hand
    //     //     .grip(right_hand_grabbed, &self.renderer.vulkan_context)?;

    //     Ok(())
    // }

    #[cfg(target_os = "android")]
    pub fn process_android_events(&mut self) {
        loop {
            if let Some(event) = self.poll_android_events() {
                println!("[HOTHAM_ANDROID] Received event {:?}", event);
                match event {
                    ndk_glue::Event::Resume => self.resumed = true,
                    ndk_glue::Event::Destroy => self.should_quit.store(true, Ordering::Relaxed),
                    ndk_glue::Event::Pause => self.resumed = false,
                    _ => {}
                }
            }
            break;
        }
    }

    #[cfg(target_os = "android")]
    pub fn poll_android_events(&mut self) -> Option<ndk_glue::Event> {
        use ndk::looper::{Poll, ThreadLooper};

        let looper = ThreadLooper::for_thread().unwrap();
        let timeout = if self.resumed {
            ANDROID_LOOPER_NONBLOCKING_TIMEOUT
        } else {
            ANDROID_LOOPER_BLOCKING_TIMEOUT
        };
        let result = looper.poll_all_timeout(timeout);

        match result {
            Ok(Poll::Event { ident, .. }) => {
                let ident = ident as u32;
                if ident == ANDROID_LOOPER_ID_MAIN {
                    ndk_glue::poll_events()
                } else if ident == ANDROID_LOOPER_ID_INPUT {
                    if let Some(input_queue) = ndk_glue::input_queue().as_ref() {
                        while let Some(event) = input_queue.get_event() {
                            if let Some(event) = input_queue.pre_dispatch(event) {
                                input_queue.finish_event(event, false);
                            }
                        }
                    }
                    None
                } else {
                    unreachable!(
                        "Unrecognised looper identifier: {:?} but LOOPER_ID_INPUT is {:?}",
                        ident, ANDROID_LOOPER_ID_INPUT
                    );
                }
            }
            _ => None,
        }
    }
}

// TODO: Add sounds to legion
// use kira::{
//     arrangement::{Arrangement, LoopArrangementSettings},
//     instance::InstanceSettings,
//     manager::{AudioManager, AudioManagerSettings},
//     sequence::{Sequence, SequenceInstanceSettings, SequenceSettings},
//     sound::handle::SoundHandle,
// };

// let mut audio_manager = AudioManager::new(AudioManagerSettings::default())
//     .map_err(|e| anyhow!("Error with Kira: {:?}", e))?;

// let hello = &self.sounds[0];
// let mut sequence = Sequence::<()>::new(SequenceSettings::default());
// sequence.wait(kira::Duration::Seconds(2.0));
// sequence.play(hello, InstanceSettings::default());
// let _sequence_instance_handle = self
//     .audio_manager
//     .start_sequence(sequence, SequenceInstanceSettings::default())?;

// let background = &self.sounds[1];
// let mut arrangement_handle = self
//     .audio_manager
//     .add_arrangement(Arrangement::new_loop(
//         background,
//         LoopArrangementSettings::default(),
//     ))
//     .map_err(anyhow::Error::new)?;
// arrangement_handle
//     .play(InstanceSettings::default())
//     .map_err(anyhow::Error::new)?;
