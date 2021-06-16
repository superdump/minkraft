use bevy::{
    core::Byteable,
    prelude::*,
    render::{
        render_graph::{base, RenderGraph, RenderResourcesNode},
        renderer::{RenderResource, RenderResources},
    },
};

const FOG_RENDER_NODE: &str = "fog";
pub const FOG_SETUP_SYSTEM: &str = "fog_setup";

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system().label(FOG_SETUP_SYSTEM));
    }
}

#[derive(Debug, Clone, Copy, RenderResource, RenderResources)]
#[render_resources(from_self)]
#[repr(C)]
pub struct FogConfig {
    pub color: [f32; 4],
    pub near: f32,
    pub far: f32,
}

unsafe impl Byteable for FogConfig {}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            color: [0.43, 0.35, 0.25, 1.0],
            near: 500.0,
            far: 5000.0,
        }
    }
}

pub fn setup(mut render_graph: ResMut<RenderGraph>) {
    // Add an AssetRenderResourcesNode to our Render Graph. This will bind
    // PhysicalSkyMaterial resources to our shader
    render_graph.add_system_node(
        FOG_RENDER_NODE,
        RenderResourcesNode::<FogConfig>::new(false),
    );

    // Add a Render Graph edge connecting our new FOG_RENDER_NODE node
    // to the main pass node. This ensures FOG_RENDER_NODE runs before
    // the main pass.
    render_graph
        .add_node_edge(FOG_RENDER_NODE, base::node::MAIN_PASS)
        .unwrap();
}
