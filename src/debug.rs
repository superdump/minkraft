use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_character_controller::look::MouseSettings;

pub struct Debug {
    pub enabled: bool,
    font_handle: Option<Handle<Font>>,
    pub text_entity: Option<Entity>,
    pub transparent_material: Option<Handle<ColorMaterial>>,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            enabled: false,
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

pub struct DebugTransformTag;

fn debug_setup(
    mut debug: ResMut<Debug>,
    asset_server: Res<AssetServer>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    debug.font_handle = Some(asset_server.load("fonts/FiraMono-Medium.ttf"));
    debug.transparent_material = Some(color_materials.add(ColorMaterial::color(Color::NONE)));
}

fn debug_toggle_system(mut commands: Commands, mut debug: ResMut<Debug>) {
    if debug.enabled {
        if debug.text_entity.is_none() {
            debug.text_entity = Some(
                commands
                    .spawn_bundle(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexEnd,
                            padding: Rect::all(Val::Px(16.0f32)),
                            ..Default::default()
                        },
                        material: debug.transparent_material.as_ref().unwrap().clone(),
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            text: Text::with_section(
                                "FT:".to_string(),
                                TextStyle {
                                    font: debug.font_handle.as_ref().unwrap().clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                        p.spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            text: Text::with_section(
                                "XYZ:".to_string(),
                                TextStyle {
                                    font: debug.font_handle.as_ref().unwrap().clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                        p.spawn_bundle(TextBundle {
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            text: Text::with_section(
                                "YP:".to_string(),
                                TextStyle {
                                    font: debug.font_handle.as_ref().unwrap().clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                    ..Default::default()
                                },
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                    })
                    .id(),
            );
        }
    } else if let Some(entity) = debug.text_entity {
        commands.entity(entity).despawn_recursive();
        debug.text_entity = None;
    }
}

fn debug_system(
    debug: Res<Debug>,
    diagnostics: Res<Diagnostics>,
    settings: Res<MouseSettings>,
    camera: Query<&Transform, With<DebugTransformTag>>,
    mut query: Query<&mut Text>,
) {
    if !debug.enabled || debug.text_entity.is_none() {
        return;
    }
    let mut cam_iter = camera.iter();
    let cam_transform = cam_iter.next().unwrap();
    for mut text in query.iter_mut() {
        match text.sections[0].value.get(..3) {
            Some("FT:") => {
                if let Some(frame_time) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
                    if let Some(fdt) = frame_time.average() {
                        text.sections[0].value =
                            format!("FT: {:6.2}ms {:6.2}fps", fdt * 1000.0f64, 1.0f64 / fdt);
                    }
                }
            }
            Some("XYZ") => {
                let cam_pos = cam_transform.translation;
                text.sections[0].value = format!(
                    "XYZ: ({:>8.2}, {:>8.2}, {:>8.2})",
                    cam_pos.x, cam_pos.y, cam_pos.z
                );
            }
            Some("YP:") => {
                text.sections[0].value = format!(
                    "YP: ({:>8.2}, {:>8.2})",
                    settings.yaw_pitch_roll.x, settings.yaw_pitch_roll.y
                );
            }
            _ => {}
        }
    }
}
