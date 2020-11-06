use bevy::{prelude::*, wgpu::diagnostic::WgpuResourceDiagnosticsPlugin};

pub struct DiagnosticPlugins;

impl PluginGroup for DiagnosticPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(FrameTimeDiagnosticsPlugin)
            .add(WgpuResourceDiagnosticsPlugin)
            .add(PrintDiagnosticsPlugin);
    }
}
