use bevy::prelude::*;
use bevy_mod_props::Identity;
use bevy_trill::{LoadResponseEngine, RequestResponse, Response, TrillPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TrillPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, idle_response)
        .run();
}

#[derive(Component)]
struct Npc {
    speach_timer: Timer,
}

fn setup(mut commands: Commands) {
    // Load the dialog system
    commands.write_message(LoadResponseEngine::default().add_source_path("dialog.trl"));

    // Create an NPC entity namd "clippy" that will say something every five seconds
    commands
        .spawn((
            Identity::new("clippy"),
            Npc {
                speach_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            },
        ))
        .observe(|response: On<Response>| {
            if let Some(line) = response.get("line") {
                println!("[clippy]: {}", line)
            }
        });
}

fn idle_response(time: Res<Time>, npcs: Query<(Entity, &mut Npc)>, mut commands: Commands) {
    for (entity, mut npc) in npcs {
        npc.speach_timer.tick(time.delta());

        if npc.speach_timer.just_finished() {
            commands.write_message(RequestResponse::new(entity, "idle"));
        }
    }
}
