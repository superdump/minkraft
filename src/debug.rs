use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_fly_camera::FlyCamera;

pub struct Debug {
    pub enabled: bool,
    font_handle: Option<Handle<Font>>,
    pub text_entity: Option<Entity>,
    pub transparent_material: Option<Handle<ColorMaterial>>,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            enabled: true,
            font_handle: None,
            text_entity: None,
            transparent_material: None,
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Debug>()
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(debug_setup.system())
            .add_system(debug_toggle_system.system())
            .add_system(debug_system.system());
    }
}

pub struct DebugCameraTag;

fn debug_setup(
    mut debug: ResMut<Debug>,
    asset_server: Res<AssetServer>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    debug.font_handle = Some(
        asset_server
            .load("assets/fonts/FiraMono-Medium.ttf")
            .unwrap(),
    );
    debug.transparent_material = Some(color_materials.add(ColorMaterial::color(Color::NONE)));
}

fn debug_toggle_system(mut commands: Commands, mut debug: ResMut<Debug>) {
    if debug.enabled {
        if debug.text_entity.is_none() {
            debug.text_entity = commands
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                        padding: Rect::all(Val::Px(16.0f32)),
                        ..Default::default()
                    },
                    material: debug.transparent_material.unwrap(),
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
                            font: debug.font_handle.unwrap(),
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
                            font: debug.font_handle.unwrap(),
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
                            font: debug.font_handle.unwrap(),
                            style: TextStyle {
                                font_size: 24.0,
                                color: Color::WHITE,
                            },
                        },
                        ..Default::default()
                    });
                })
                .current_entity();
        }
    } else if let Some(entity) = debug.text_entity {
        commands.despawn_recursive(entity);
        debug.text_entity = None;
    }
}

fn debug_system(
    debug: Res<Debug>,
    diagnostics: Res<Diagnostics>,
    mut camera: Query<With<DebugCameraTag, (&Transform, &FlyCamera)>>,
    mut query: Query<&mut Text>,
) {
    if !debug.enabled || debug.text_entity.is_none() {
        return;
    }
    let mut cam_iter = camera.iter();
    let (cam_transform, fly_cam) = cam_iter.iter().next().unwrap();
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
