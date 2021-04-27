use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
    render::draw::OutsideFrustum,
};

/// Adds "frame time" diagnostic to an App, specifically "frame time", "fps" and "frame count"
#[derive(Default)]
pub struct MeshDiagnosticsPlugin;

impl Plugin for MeshDiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(Self::setup_system.system())
            .add_system(Self::diagnostic_system.system());
    }
}

impl MeshDiagnosticsPlugin {
    pub const MESH_ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(80771266489901194674757474585028451990);
    pub const CULLED_MESH_ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(195344731070922658119191847003798465292);
    pub const DRAWN_MESH_ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(332418629918566815433557878873025708821);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(
            Self::MESH_ENTITY_COUNT,
            "mesh_entity_count",
            1,
        ));
        diagnostics.add(Diagnostic::new(
            Self::CULLED_MESH_ENTITY_COUNT,
            "culled_mesh_entity_count",
            1,
        ));
        diagnostics.add(Diagnostic::new(
            Self::DRAWN_MESH_ENTITY_COUNT,
            "drawn_mesh_entity_count",
            1,
        ));
    }

    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        query: Query<&Visible, With<Handle<Mesh>>>,
    ) {
        let (mut culled_mesh_count, mut drawn_mesh_count) = (0.0, 0.0);
        for visible in query.iter() {
            if visible.is_visible {
                drawn_mesh_count += 1.0;
            } else {
                culled_mesh_count += 1.0;
            }
        }
        diagnostics.add_measurement(
            Self::MESH_ENTITY_COUNT,
            culled_mesh_count + drawn_mesh_count,
        );
        diagnostics.add_measurement(Self::CULLED_MESH_ENTITY_COUNT, culled_mesh_count);
        diagnostics.add_measurement(Self::DRAWN_MESH_ENTITY_COUNT, drawn_mesh_count);
    }
}
