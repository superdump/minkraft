#[cfg(feature = "profiler")]
use bevy::diagnostic::PrintDiagnosticsPlugin;
use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_rapier3d::{
    physics::{RapierPhysicsPlugin, RigidBodyHandleComponent},
    rapier::dynamics::{RigidBodyBuilder, RigidBodySet},
    rapier::geometry::ColliderBuilder,
};
use minkraft::{
    character_controller::*,
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
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(FlyCameraPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(WorldAxesPlugin)
        .add_plugin(GeneratePlugin)
        .add_plugin(CharacterControllerPlugin)
        .add_startup_system(setup_world.system())
        .add_startup_system(setup_player.system())
        .add_system(physics_input.system())
        .add_system(exit_on_esc_system.system())
        .add_system(enable_fly_camera.system())
        .add_system(toggle_debug_system.system());

    #[cfg(feature = "profiler")]
    app_builder.add_plugin(PrintDiagnosticsPlugin::default());

    app_builder.run();
}

pub struct PlayerTag;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_pos = Vec3::new(1.1, 65.0, 1.1);
    let obj_scale = Vec3::new(0.465, 1.75, 0.25);

    let eye = Vec3::new(0.0, 4.0, 8.0);
    let center = Vec3::zero();
    let camera_transform = Mat4::face_toward(eye, center, Vec3::unit_y());

    // FIXME: Hacks to sync the FlyCamera with the camera_transform
    let eye_center = (center - eye).normalize();
    let pitch = eye_center.y().asin();
    let yaw = eye_center.z().atan2(eye_center.x());

    let red = materials.add(Color::hex("DC143C").unwrap().into());
    let cuboid = meshes.add(Mesh::from(shape::Cube::default()));

    commands
        .spawn((
            GlobalTransform::identity(),
            Transform::from_translation(spawn_pos),
        ))
        .with(RigidBodyBuilder::new_kinematic().translation(
            spawn_pos.x(),
            spawn_pos.y(),
            spawn_pos.z(),
        ))
        .with(ColliderBuilder::cuboid(
            obj_scale.x(),
            obj_scale.y(),
            obj_scale.z(),
        ))
        .with(CharacterController {
            pitch: -pitch.to_degrees(),
            yaw: yaw.to_degrees() - 180.0f32,
            ..Default::default()
        })
        .with(PlayerTag)
        .with(GeneratedVoxelsTag)
        .with(CharacterControllerBodyTag)
        .with_children(|body| {
            body.spawn(PbrComponents {
                material: red,
                mesh: cuboid,
                transform: Transform::new(Mat4::from_scale(obj_scale)),
                ..Default::default()
            })
            .spawn((
                GlobalTransform::identity(),
                Transform::new(Mat4::from_translation(
                    0.8 * 0.5 * obj_scale.y() * Vec3::unit_y(),
                )),
                CharacterControllerHeadTag,
            ))
            .with_children(|head| {
                head.spawn(Camera3dComponents {
                    transform: Transform::new(camera_transform),
                    ..Default::default()
                })
                .with(DebugCameraTag)
                .with(WorldAxesCameraTag)
                .with(CharacterControllerCameraTag);
            });
        });
}

fn setup_world(mut commands: Commands) {
    commands
        .spawn(UiCameraComponents::default())
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(14.0, 18.0, 14.0)),
            ..Default::default()
        });
}

fn physics_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut rigid_bodies: ResMut<RigidBodySet>,
    mut player_query: Query<(&PlayerTag, &RigidBodyHandleComponent)>,
) {
    let mut player_temp = player_query.iter();
    let (_, player_index) = player_temp.iter().next().unwrap();
    let mut player = rigid_bodies.get_mut(player_index.handle()).unwrap();
    let force_multiplier = 2.0;
    if keyboard_input.pressed(KeyCode::Up) {
        player.wake_up();
        player.apply_impulse([0.0, 0.0, force_multiplier].into());
    }
    if keyboard_input.pressed(KeyCode::Down) {
        player.wake_up();
        player.apply_impulse([0.0, 0.0, -force_multiplier].into());
    }
    if keyboard_input.pressed(KeyCode::Right) {
        player.wake_up();
        player.apply_impulse([force_multiplier, 0.0, 0.0].into());
    }
    if keyboard_input.pressed(KeyCode::Left) {
        player.wake_up();
        player.apply_impulse([-force_multiplier, 0.0, 0.0].into());
    }
    if keyboard_input.pressed(KeyCode::Space) {
        player.wake_up();
        player.apply_impulse([0.0, 3.0 * force_multiplier, 0.0].into());
    }
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
