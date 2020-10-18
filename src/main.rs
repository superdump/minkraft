#[cfg(feature = "profiler")]
use bevy::diagnostic::PrintDiagnosticsPlugin;
use bevy::{
    input::{keyboard::KeyCode, system::exit_on_esc_system},
    prelude::*,
};
use bevy_prototype_character_controller::{
    controller::{BodyTag, CameraTag, CharacterController, HeadTag, YawTag},
    look::LookDirection,
    rapier::RapierDynamicImpulseCharacterControllerPlugin,
};
use bevy_rapier3d::{
    physics::{PhysicsInterpolationComponent, RapierPhysicsPlugin},
    rapier::dynamics::RigidBodyBuilder,
    rapier::geometry::ColliderBuilder,
};
use minkraft::{
    debug::{Debug, DebugPlugin, DebugTransformTag},
    generate::{GeneratePlugin, GeneratedVoxelsTag},
    world_axes::{WorldAxes, WorldAxesCameraTag, WorldAxesPlugin},
};

fn main() {
    env_logger::builder().format_timestamp_micros().init();

    let mut app_builder = App::build();
    app_builder
        // Generic
        .add_resource(WindowDescriptor {
            title: env!("CARGO_PKG_NAME").to_string(),
            ..Default::default()
        })
        .add_resource(ClearColor(Color::BLACK))
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_system(exit_on_esc_system.system())
        // Debug
        .add_plugin(DebugPlugin)
        .add_system_to_stage(bevy::app::stage::PRE_UPDATE, toggle_debug_system.system())
        .add_plugin(WorldAxesPlugin)
        // Physics - Rapier
        .add_plugin(RapierPhysicsPlugin)
        // Character Controller
        .add_plugin(RapierDynamicImpulseCharacterControllerPlugin)
        // Terrain
        .add_plugin(GeneratePlugin)
        // Minkraft
        .add_startup_system(setup_world.system())
        .add_startup_system(setup_player.system());

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
    let spawn_pos = Vec3::new(1.1, 90.0, 1.1);
    let obj_scale = Vec3::new(0.465, 1.75, 0.25);

    let eye = Vec3::new(0.0, 4.0, 8.0);
    let center = Vec3::zero();
    let camera_transform = Mat4::face_toward(eye, center, Vec3::unit_y());

    let red = materials.add(Color::hex("DC143C").unwrap().into());
    let cuboid = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));

    commands
        .spawn((
            GlobalTransform::identity(),
            Transform::from_translation(spawn_pos),
            RigidBodyBuilder::new_dynamic().translation(
                spawn_pos.x(),
                spawn_pos.y(),
                spawn_pos.z(),
            ),
            ColliderBuilder::cuboid(
                0.5 * obj_scale.x(),
                0.5 * obj_scale.y(),
                0.5 * obj_scale.z(),
            )
            .density(200.0),
            PhysicsInterpolationComponent::new(spawn_pos, Quat::identity()),
            CharacterController::default(),
            BodyTag,
            PlayerTag,
            GeneratedVoxelsTag,
            DebugTransformTag,
        ))
        .with_children(|body| {
            body.spawn((GlobalTransform::identity(), Transform::identity(), YawTag))
                .with_children(|yaw| {
                    yaw.spawn(PbrComponents {
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
                        HeadTag,
                    ))
                    .with_children(|head| {
                        head.spawn(PbrComponents {
                            material: red,
                            mesh: cuboid,
                            transform: Transform::from_scale(0.3),
                            ..Default::default()
                        })
                        .spawn(Camera3dComponents {
                            transform: Transform::new(camera_transform),
                            ..Default::default()
                        })
                        .with_bundle((
                            LookDirection::default(),
                            CameraTag,
                            WorldAxesCameraTag,
                        ));
                    });
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
