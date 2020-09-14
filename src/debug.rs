use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

#[derive(Default)]
struct Debug(bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<Debug>()
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_startup_system(debug_setup.system())
            .add_system(debug_system.system());
    }
}

fn debug_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    commands
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                value: "FPS:".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}

fn debug_system(
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text>,
) {
    for mut text in &mut query.iter() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                text.value = format!("FPS: {:.2}", average);
            }
        }
    }
}
