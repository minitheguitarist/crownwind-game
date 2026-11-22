use bevy::prelude::*;
use bevy::window::WindowResolution;
use game_bevy::GamePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Crownwind \u{2014} Windwood (preview)".into(),
                        // 2× the 480×270 virtual viewport: integer scaling.
                        resolution: WindowResolution::new(960, 540),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    // The workspace `assets/` folder. Resolved against
                    // CARGO_MANIFEST_DIR (crates/game_app) under `cargo run`,
                    // or the executable's folder (target/<profile>) otherwise;
                    // both are two levels below the workspace root.
                    // BEVY_ASSET_ROOT overrides the base folder.
                    file_path: "../../assets".to_string(),
                    ..default()
                }),
        )
        .add_plugins(GamePlugin)
        .run();
}
