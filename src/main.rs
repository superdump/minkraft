use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use minkraft::{debug::DebugPlugin, generate::*, types::CameraTag};

#[derive(Default)]
struct Debug(bool);

fn main() {
    env_logger::builder().format_timestamp_micros().init();
    App::build()
        .add_resource(WindowDescriptor {
            title: env!("CARGO_PKG_NAME").to_string(),
            ..Default::default()
        })
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(DebugPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(GeneratePlugin)
        .add_startup_system(setup_world.system())
        .add_system(exit_on_esc_system.system())
        .run();
}

fn setup_world(mut commands: Commands) {
    commands
        .spawn(UiCameraComponents::default())
        .spawn(LightComponents {
            translation: Translation::new(14.0, 18.0, 14.0),
            ..Default::default()
        })
        .spawn(Camera3dComponents {
            translation: Vec3::new(-30f32, 65f32, -30f32).into(),
            ..Default::default()
        })
        .with(FlyCamera {
            pitch: 40.0f32,
            yaw: -135.0f32,
            key_up: KeyCode::Q,
            key_down: KeyCode::E,
            ..Default::default()
        })
        .with(CameraTag);
}
