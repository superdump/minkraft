use crate::{shapes::*, types::CameraTag};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::camera::Camera,
};
use bevy_fly_camera::FlyCamera;

pub struct Debug {
    pub enabled: bool,
    pub screen_position: Vec3,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            enabled: true,
            screen_position: Vec3::new(0.95, 0.95, 0.3),
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Debug>()
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(debug_setup.system())
            .add_system(debug_system.system())
            .add_system(axes_system.system());
    }
}

pub struct AxesTag;
pub struct AxesCameraTag;

fn axes_system(
    debug: Res<Debug>,
    mut camera_query: Query<(&Camera, &Transform)>,
    mut axes_query: Query<(&AxesTag, &mut Transform)>,
) {
    let mut cam_temp = camera_query.iter();
    let (camera, camera_transform) = cam_temp.iter().next().unwrap();
    let mut axes_temp = axes_query.iter();
    let (_, mut axes_transform) = axes_temp.iter().next().unwrap();

    let view_matrix = camera_transform.value();
    let projection_matrix = camera.projection_matrix;
    let world_pos: Vec4 =
        (*view_matrix * projection_matrix.inverse()).mul_vec4(debug.screen_position.extend(1.0));
    let position: Vec3 = (world_pos / world_pos.w()).truncate().into();

    axes_transform.set_translation(position);
}

fn debug_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    // In-scene
    let cylinder_mesh = meshes.add(Mesh::from(Cylinder {
        height: 0.85f32,
        radius: 0.03f32,
        ..Default::default()
    }));
    let cone_mesh = meshes.add(Mesh::from(Cone {
        height: 0.15f32,
        radius: 0.07f32,
        ..Default::default()
    }));
    let red = standard_materials.add(Color::RED.into());
    let green = standard_materials.add(Color::GREEN.into());
    let blue = standard_materials.add(Color::BLUE.into());

    commands
        .spawn((
            GlobalTransform::identity(),
            Transform::from_scale(0.1f32),
            AxesTag,
        ))
        .with_children(|axes_root| {
            axes_root
                .spawn((
                    GlobalTransform::identity(),
                    Transform::from_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
                ))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: red,
                            mesh: cone_mesh,
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        })
                        .spawn(PbrComponents {
                            material: red,
                            mesh: cylinder_mesh,
                            ..Default::default()
                        });
                })
                .spawn((GlobalTransform::identity(), Transform::identity()))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: green,
                            mesh: cone_mesh,
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        })
                        .spawn(PbrComponents {
                            material: green,
                            mesh: cylinder_mesh,
                            ..Default::default()
                        });
                })
                .spawn((
                    GlobalTransform::identity(),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: blue,
                            mesh: cone_mesh,
                            transform: Transform::from_translation(Vec3::new(
                                0.0f32, 0.85f32, 0.0f32,
                            )),
                            ..Default::default()
                        })
                        .spawn(PbrComponents {
                            material: blue,
                            mesh: cylinder_mesh,
                            ..Default::default()
                        });
                });
        });

    // UI
    let font_handle = asset_server
        .load("assets/fonts/FiraMono-Medium.ttf")
        .unwrap();
    commands
        .spawn(NodeComponents {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                padding: Rect::all(Val::Px(16.0f32)),
                ..Default::default()
            },
            material: color_materials.add(ColorMaterial::color(Color::NONE)),
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn(TextComponents {
                style: Style {
                    align_self: AlignSelf::FlexStart,
                    ..Default::default()
                },
                text: Text {
                    value: "FT:".to_string(),
                    font: font_handle,
                    style: TextStyle {
                        font_size: 24.0,
                        color: Color::WHITE,
                    },
                },
                ..Default::default()
            })
            .spawn(TextComponents {
                style: Style {
                    align_self: AlignSelf::FlexStart,
                    ..Default::default()
                },
                text: Text {
                    value: "XYZ:".to_string(),
                    font: font_handle,
                    style: TextStyle {
                        font_size: 24.0,
                        color: Color::WHITE,
                    },
                },
                ..Default::default()
            })
            .spawn(TextComponents {
                style: Style {
                    align_self: AlignSelf::FlexStart,
                    ..Default::default()
                },
                text: Text {
                    value: "YP:".to_string(),
                    font: font_handle,
                    style: TextStyle {
                        font_size: 24.0,
                        color: Color::WHITE,
                    },
                },
                ..Default::default()
            });
        });
}

fn debug_system(
    diagnostics: Res<Diagnostics>,
    mut camera: Query<(&CameraTag, &Transform, &FlyCamera)>,
    mut query: Query<&mut Text>,
) {
    let mut cam_iter = camera.iter();
    let (_, cam_transform, fly_cam) = cam_iter.into_iter().next().unwrap();
    for mut text in &mut query.iter() {
        match text.value.get(..3) {
            Some("FT:") => {
                if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
                    if let Some(fdt) = frame_time.average() {
                        text.value =
                            format!("FT: {:6.2}ms {:6.2}fps", fdt * 1000.0f64, 1.0f64 / fdt);
                    }
                }
            }
            Some("XYZ") => {
                let cam_pos = cam_transform.translation();
                text.value = format!(
                    "XYZ: ({:>8.2}, {:>8.2}, {:>8.2})",
                    cam_pos.x(),
                    cam_pos.y(),
                    cam_pos.z()
                );
            }
            Some("YP:") => {
                text.value = format!("YP: ({:>8.2}, {:>8.2})", fly_cam.yaw, fly_cam.pitch);
            }
            _ => {}
        }
    }
}
