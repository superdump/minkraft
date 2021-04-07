use crate::shapes::{Cone, Cylinder};
use bevy::{prelude::*, render::camera::Camera};

pub struct WorldAxesPlugin;

pub const UPDATE_AXES_TRANSFORM: &str = "update_axes_transform";

impl Plugin for WorldAxesPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<WorldAxes>()
            .add_startup_system(world_axes_setup_system.system())
            .add_system_to_stage(
                bevy::app::CoreStage::PreUpdate,
                world_axes_toggle_system.system(),
            )
            .add_stage_after(
                bevy::app::CoreStage::Update,
                UPDATE_AXES_TRANSFORM,
                SystemStage::parallel(),
            )
            .add_system_to_stage(UPDATE_AXES_TRANSFORM, world_axes_system.system());
    }
}

pub struct WorldAxes {
    pub enabled: bool,
    pub clip_space_position: Vec3,
    pub scale: f32,
    pub axes_entity: Option<Entity>,
    pub meshes: Vec<Handle<Mesh>>,
    pub standard_materials: Vec<Handle<StandardMaterial>>,
}

impl Default for WorldAxes {
    fn default() -> Self {
        WorldAxes {
            enabled: false,
            clip_space_position: Vec3::new(0.73044837, -0.59729564, 0.2318211),
            scale: 0.1f32,
            axes_entity: None,
            meshes: Vec::with_capacity(2),
            standard_materials: Vec::with_capacity(3),
        }
    }
}

pub struct WorldAxesTag;
pub struct WorldAxesCameraTag;

fn world_axes_setup_system(
    mut world_axes: ResMut<WorldAxes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    let cylinder_mesh = meshes.add(Mesh::from(Cylinder {
        height: 0.85f32,
        radius: 0.03f32,
        ..Default::default()
    }));
    world_axes.meshes.push(cylinder_mesh);
    let cone_mesh = meshes.add(Mesh::from(Cone {
        height: 0.15f32,
        radius: 0.07f32,
        ..Default::default()
    }));
    world_axes.meshes.push(cone_mesh);

    let red = standard_materials.add(Color::RED.into());
    world_axes.standard_materials.push(red);
    let green = standard_materials.add(Color::GREEN.into());
    world_axes.standard_materials.push(green);
    let blue = standard_materials.add(Color::BLUE.into());
    world_axes.standard_materials.push(blue);
}

fn spawn_world_axes(commands: &mut Commands, world_axes: &mut ResMut<WorldAxes>) {
    let red = world_axes.standard_materials[0].clone();
    let green = world_axes.standard_materials[1].clone();
    let blue = world_axes.standard_materials[2].clone();

    let cylinder_mesh = world_axes.meshes[0].clone();
    let cone_mesh = world_axes.meshes[1].clone();

    world_axes.axes_entity = Some(
        commands
            .spawn_bundle((
                GlobalTransform::identity(),
                Transform::from_scale(Vec3::splat(world_axes.scale)),
                WorldAxesTag,
            ))
            .with_children(|axes_root| {
                axes_root
                    .spawn_bundle((
                        GlobalTransform::identity(),
                        Transform::from_rotation(Quat::from_rotation_z(
                            -std::f32::consts::FRAC_PI_2,
                        )),
                    ))
                    .with_children(|axis_root| {
                        axis_root.spawn_bundle(PbrBundle {
                            material: red.clone(),
                            mesh: cone_mesh.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        });
                        axis_root.spawn_bundle(PbrBundle {
                            material: red,
                            mesh: cylinder_mesh.clone(),
                            ..Default::default()
                        });
                    });
                axes_root
                    .spawn_bundle((GlobalTransform::identity(), Transform::identity()))
                    .with_children(|axis_root| {
                        axis_root.spawn_bundle(PbrBundle {
                            material: green.clone(),
                            mesh: cone_mesh.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        });
                        axis_root.spawn_bundle(PbrBundle {
                            material: green,
                            mesh: cylinder_mesh.clone(),
                            ..Default::default()
                        });
                    });
                axes_root
                    .spawn_bundle((
                        GlobalTransform::identity(),
                        Transform::from_rotation(Quat::from_rotation_x(
                            std::f32::consts::FRAC_PI_2,
                        )),
                    ))
                    .with_children(|axis_root| {
                        axis_root.spawn_bundle(PbrBundle {
                            material: blue.clone(),
                            mesh: cone_mesh,
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        });
                        axis_root.spawn_bundle(PbrBundle {
                            material: blue,
                            mesh: cylinder_mesh,
                            ..Default::default()
                        });
                    });
            })
            .id(),
    );
}

fn world_axes_toggle_system(mut commands: Commands, mut world_axes: ResMut<WorldAxes>) {
    if world_axes.enabled {
        if world_axes.axes_entity.is_none() {
            spawn_world_axes(&mut commands, &mut world_axes);
        }
    } else if let Some(entity) = world_axes.axes_entity {
        commands.entity(entity).despawn_recursive();
        world_axes.axes_entity = None;
    }
}

// NOTE: This system depends on the tagged camera's GlobalTransform having been updated!
fn world_axes_system(
    world_axes: Res<WorldAxes>,
    camera_query: Query<(&Camera, &GlobalTransform), With<WorldAxesCameraTag>>,
    mut axes_query: Query<&mut Transform, With<WorldAxesTag>>,
) {
    if !world_axes.enabled || world_axes.axes_entity.is_none() {
        return;
    }
    let mut cam_temp = camera_query.iter();
    let (camera, camera_transform) = cam_temp.next().unwrap();
    let mut axes_temp = axes_query.iter_mut();
    let mut axes_transform = axes_temp.next().unwrap();

    let inv_view_matrix = camera_transform.compute_matrix();
    let projection_matrix = camera.projection_matrix;
    let world_pos: Vec4 = (inv_view_matrix * projection_matrix.inverse())
        .mul_vec4(world_axes.clip_space_position.extend(1.0));
    let position: Vec3 = (world_pos / world_pos.w).truncate().into();

    axes_transform.translation = position;
}
