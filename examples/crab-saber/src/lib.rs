mod components;
mod game_context;
mod systems;

use hotham::{
    components::Visible,
    hecs::{Entity, World},
    systems::{
        audio_system, draw_gui_system, haptics_system, physics_system, pointers_system,
        rendering_system, update_global_transform_system,
        update_global_transform_with_parent_system,
    },
    xr::{self, SessionState},
    Engine, HothamResult, TickData,
};

use game_context::{GameContext, GameState};
use systems::{game::game_system, sabers_system};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[CRAB_SABER] MAIN!");
    real_main().expect("[CRAB_SABER] ERROR IN MAIN!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut game_context = init(&mut engine);

    while let Ok(tick_data) = engine.update() {
        tick(tick_data, &mut engine, &mut game_context);
        engine.finish()?;
    }

    Ok(())
}

fn tick(tick_data: TickData, engine: &mut Engine, game_context: &mut GameContext) {
    handle_state_change(&tick_data, engine, game_context);

    // Simulation tasks - these are only necessary in the focussed state.
    if tick_data.current_state == xr::SessionState::FOCUSED {
        // Sync world with input contexts
        sabers_system(engine);
        pointers_system(engine);

        // Update physics simulation
        physics_system(engine);

        // Update game simulation
        game_system(engine, game_context);

        // Update world
        update_global_transform_with_parent_system(engine);
        update_global_transform_system(engine);

        // Sync world with output contexts
        haptics_system(engine);
        audio_system(engine);
        draw_gui_system(engine);
    }

    // Draw objects
    rendering_system(engine, tick_data.swapchain_image_index);
}

fn handle_state_change(tick_data: &TickData, engine: &mut Engine, game_context: &mut GameContext) {
    let world = &mut engine.world;
    let audio_context = &mut engine.audio_context;
    match (tick_data.previous_state, tick_data.current_state) {
        (SessionState::VISIBLE, SessionState::FOCUSED) => {
            audio_context.resume_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    show(world, game_context.pointer);
                }
                GameState::Playing(_) => {
                    show(world, game_context.blue_saber);
                    show(world, game_context.red_saber);
                }
            }
        }
        (SessionState::FOCUSED, SessionState::VISIBLE) => {
            audio_context.pause_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    hide(world, game_context.pointer);
                }
                GameState::Playing(_) => {
                    hide(world, game_context.blue_saber);
                    hide(world, game_context.red_saber);
                }
            }
        }
        _ => {}
    }
}

fn init(engine: &mut Engine) -> GameContext {
    let mut game_context = GameContext::new(engine);
    game_context.add_songs(&mut engine.audio_context);
    game_context.add_sound_effects(&mut engine.audio_context);
    game_context
}

fn hide(world: &mut World, entity: Entity) {
    if world.remove_one::<Visible>(entity).is_err() {
        println!("[STATE_CHANGE] Tried to make {entity:?} hidden but it had no Visible component")
    }
}

fn show(world: &mut World, entity: Entity) {
    world.insert_one(entity, Visible {}).unwrap();
}
