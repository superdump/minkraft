use crate::types::CameraTag;
use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_fly_camera::FlyCamera;

#[derive(Default)]
struct Debug(bool);

const RADIANS_TO_DEGREES: f32 = 180.0 / std::f32::consts::PI;
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
    let cube_mesh = meshes.add(Mesh::from(shape::Cube::default()));
    commands
        .spawn((Transform::identity(),))
        .with_children(|parent| {
            parent
                .spawn(PbrComponents {
                    material: standard_materials.add(Color::RED.into()),
                    mesh: cube_mesh,
                    transform: Transform::new_sync_disabled(Mat4::from_scale(Vec3::new(
                        1.0f32, 0.1f32, 0.1f32,
                    ))),
                    ..Default::default()
                })
                .spawn(PbrComponents {
                    material: standard_materials.add(Color::BLUE.into()),
                    mesh: cube_mesh,
                    transform: Transform::new_sync_disabled(Mat4::from_scale(Vec3::new(
                        0.1f32, 1.0f32, 0.1f32,
                    ))),
                    ..Default::default()
                })
                .spawn(PbrComponents {
                    material: standard_materials.add(Color::GREEN.into()),
                    mesh: cube_mesh,
                    transform: Transform::new_sync_disabled(Mat4::from_scale(Vec3::new(
                        0.1f32, 0.1f32, 1.0f32,
                    ))),
                    ..Default::default()
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
    mut camera: Query<(&CameraTag, &Translation, &FlyCamera)>,
    mut query: Query<&mut Text>,
) {
    let mut cam_iter = camera.iter();
    let (_, cam_pos, fly_cam) = cam_iter.into_iter().next().unwrap();
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
