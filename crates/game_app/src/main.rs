use bevy::prelude::*;
use bevy::window::WindowResolution;
use game_bevy::GamePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Crownwind (prototype)".into(),
                        // 2× the 480×270 virtual viewport: integer scaling.
                        resolution: WindowResolution::new(960, 540),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(GamePlugin)
        .run();
}
