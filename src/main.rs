use bevy::{
    input::system::exit_on_esc_system,
    prelude::*,
};
use bevy_fly_camera::{
    FlyCamera,
    FlyCameraPlugin,
};
use bevy_book::{
    debug::DebugPlugin,
    generate::*,
};

#[derive(Default)]
struct Debug(bool);

fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(DebugPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(GeneratePlugin)
        .add_startup_system(setup_world.system())
        .add_system(exit_on_esc_system.system())
        .run();
}

fn setup_world(
    mut commands: Commands,
) {
    commands
    .spawn(UiCameraComponents::default())
    .spawn(LightComponents {
            translation: Translation::new(14.0, 18.0, 14.0),
            ..Default::default()
        })
        .spawn(Camera3dComponents {
            translation: Vec3::new(10f32, 10f32, -10f32).into(),
            ..Default::default()
        })
        .with(FlyCamera::default());
}
