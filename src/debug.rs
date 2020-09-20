use crate::{shapes::*, types::CameraTag};
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_fly_camera::FlyCamera;

#[derive(Default)]
struct Debug(bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Debug>()
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(debug_setup.system())
            .add_system(debug_system.system());
    }
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
        .spawn((Transform::identity(),))
        .with_children(|axes_root| {
            axes_root
                .spawn((Transform::identity(),))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: red,
                            mesh: cone_mesh,
                            global_transform: GlobalTransform::from_translation_rotation(
                                Vec3::new(0.85f32, 0.0f32, 0.0f32),
                                Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                            ),
                            ..Default::default()
                        })
                        .spawn(PbrComponents {
                            material: red,
                            mesh: cylinder_mesh,
                            global_transform: GlobalTransform::from_rotation(
                                Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                            ),
                            ..Default::default()
                        });
                })
                .spawn((Transform::identity(),))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: green,
                            mesh: cone_mesh,
                            global_transform: GlobalTransform::from_translation(Vec3::new(
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
                .spawn((Transform::identity(),))
                .with_children(|axis_root| {
                    axis_root
                        .spawn(PbrComponents {
                            material: blue,
                            mesh: cone_mesh,
                            global_transform: GlobalTransform::from_translation_rotation(
                                Vec3::new(0.0f32, 0.0f32, 0.85f32),
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ),
                            ..Default::default()
                        })
                        .spawn(PbrComponents {
                            material: blue,
                            mesh: cylinder_mesh,
                            global_transform: GlobalTransform::from_rotation(
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                            ),
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
                    value: "FDT:".to_string(),
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
                    value: "POS:".to_string(),
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
                    value: "ROT:".to_string(),
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

fn quat_to_euler(q: &Quat) -> (f32, f32, f32) {
    // roll (x-axis rotation)
    let sinr_cosp = 2.0f32 * (q.w() * q.x() + q.y() * q.z());
    let cosr_cosp = 1.0f32 - 2.0f32 * (q.x() * q.x() + q.y() * q.y());
    let roll = sinr_cosp.atan2(cosr_cosp);

    // pitch (y-axis rotation)
    let sinp = 2.0f32 * (q.w() * q.y() - q.z() * q.x());
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp) // use 90 degrees if out of range
    } else {
        sinp.asin()
    };

    // yaw (z-axis rotation)
    let siny_cosp = 2.0f32 * (q.w() * q.z() + q.x() * q.y());
    let cosy_cosp = 1.0f32 - 2.0f32 * (q.y() * q.y() + q.z() * q.z());
    let yaw = siny_cosp.atan2(cosy_cosp);

    (roll, pitch, yaw)
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
            Some("FDT") => {
                if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
                    if let Some(fdt) = frame_time.average() {
                        text.value = format!("FDT: {:6.2}ms", fdt * 1000.0f64);
                    }
                }
            }
            Some("POS") => {
                let cam_pos = cam_transform.translation();
                text.value = format!(
                    "POS: ({:>8.2}, {:>8.2}, {:>8.2})",
                    cam_pos.x(),
                    cam_pos.y(),
                    cam_pos.z()
                );
            }
            Some("ROT") => {
                text.value = format!("ROT: ({:>8.2}, {:>8.2})", fly_cam.pitch, fly_cam.yaw);
            }
            _ => {}
        }
    }
}
