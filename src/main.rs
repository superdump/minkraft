#[cfg(feature = "profiler")]
use bevy::diagnostic::PrintDiagnosticsPlugin;
use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use minkraft::{
    debug::{Debug, DebugCameraTag, DebugPlugin},
    generate::*,
    world_axes::{WorldAxes, WorldAxesCameraTag, WorldAxesPlugin},
};

fn main() {
    env_logger::builder().format_timestamp_micros().init();

    let mut app_builder = App::build();
    app_builder
        .add_resource(WindowDescriptor {
            title: env!("CARGO_PKG_NAME").to_string(),
            ..Default::default()
        })
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(FlyCameraPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(WorldAxesPlugin)
        .add_plugin(GeneratePlugin)
        .add_startup_system(setup_world.system())
        .add_system(exit_on_esc_system.system())
        .add_system(enable_fly_camera.system())
        .add_system(toggle_debug_system.system());

    #[cfg(feature = "profiler")]
    app_builder.add_plugin(PrintDiagnosticsPlugin::default());

    app_builder.run();
}

fn setup_world(mut commands: Commands) {
    let eye = Vec3::new(0.0, 64.0, 0.0);
    let center = Vec3::new(64.0, 0.0, 64.0);
    let camera_transform = Mat4::face_toward(eye, center, Vec3::unit_y());

    // FIXME: Hacks to sync the FlyCamera with the camera_transform
    let eye_center = (center - eye).normalize();
    let pitch = eye_center.y().asin();
    let yaw = eye_center.z().atan2(eye_center.x());

    commands
        .spawn(UiCameraComponents::default())
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(14.0, 18.0, 14.0)),
            ..Default::default()
        })
        .spawn(Camera3dComponents {
            transform: Transform::new(camera_transform),
            ..Default::default()
        })
        .with(FlyCamera {
            sensitivity: 10.0f32,
            speed: 0.1f32,
            max_speed: 5.0f32,
            pitch: -pitch.to_degrees(),
            yaw: yaw.to_degrees() - 180.0f32,
            key_up: KeyCode::Q,
            key_down: KeyCode::E,
            ..Default::default()
        })
        .with(DebugCameraTag)
        .with(GeneratedVoxelsCameraTag)
        .with(WorldAxesCameraTag);
}

fn enable_fly_camera(keyboard_input: Res<Input<KeyCode>>, mut options: Mut<FlyCamera>) {
    if keyboard_input.just_pressed(KeyCode::M) {
        options.enabled = !options.enabled;
    }
}

fn toggle_debug_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut debug: ResMut<Debug>,
    mut world_axes: ResMut<WorldAxes>,
) {
    if keyboard_input.just_pressed(KeyCode::H) {
        // Use debug.state as the source of truth
        let new_state = !debug.enabled;
        debug.enabled = new_state;
        world_axes.enabled = new_state;
    }
}
