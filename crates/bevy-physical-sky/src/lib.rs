use bevy::{
    core::Byteable,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::{RenderResource, RenderResources},
        shader::ShaderDefs,
    },
};

pub const PHYSICAL_SKY_SETUP_SYSTEM: &str = "physical_sky_setup";
pub const PHYSICAL_SKY_RENDER_NODE: &str = "physical_sky";
pub const PHYSICAL_SKY_VERTEX_SHADER: &str = include_str!("../assets/shaders/physical_sky.vert");
pub const PHYSICAL_SKY_FRAGMENT_SHADER: &str = include_str!("../assets/shaders/physical_sky.frag");

pub struct PhysicalSkyPlugin;

impl Plugin for PhysicalSkyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<PhysicalSkyMaterial>()
            .add_startup_system(setup.system().label(PHYSICAL_SKY_SETUP_SYSTEM));
    }
}

#[derive(Debug, RenderResource, RenderResources, ShaderDefs, TypeUuid)]
#[uuid = "3035b6eb-0716-4980-8ed9-6d4308900e30"]
#[render_resources(from_self)]
pub struct PhysicalSkyMaterial {
    pub mie_k_coefficient: Vec4,
    pub primaries: Vec4,
    pub sun_position: Vec4,
    pub depolarization_factor: f32,
    pub luminance: f32,
    pub mie_coefficient: f32,
    pub mie_directional_g: f32,
    pub mie_v: f32,
    pub mie_zenith_length: f32,
    pub num_molecules: f32,
    pub rayleigh: f32,
    pub rayleigh_zenith_length: f32,
    pub refractive_index: f32,
    pub sun_angular_diameter_degrees: f32,
    pub sun_intensity_factor: f32,
    pub sun_intensity_falloff_steepness: f32,
    pub tonemap_weighting: f32,
    pub turbidity: f32,
}

unsafe impl Byteable for PhysicalSkyMaterial {}

// Defaults to the red sunset preset from https://tw1ddle.github.io/Sky-Shader/
impl Default for PhysicalSkyMaterial {
    fn default() -> Self {
        let mut sky = Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(6.8e-7, 5.5e-7, 4.5e-7, 0.0),
            sun_position: Vec4::ZERO,
            depolarization_factor: 0.02,
            luminance: 1.00,
            mie_coefficient: 0.005,
            mie_directional_g: 0.82,
            mie_v: 3.936,
            mie_zenith_length: 34000.0,
            num_molecules: 2.542e25,
            rayleigh: 2.28,
            rayleigh_zenith_length: 8400.0,
            refractive_index: 1.00029,
            sun_angular_diameter_degrees: 0.00933,
            sun_intensity_factor: 1000.0,
            sun_intensity_falloff_steepness: 1.5,
            tonemap_weighting: 9.50,
            turbidity: 4.7,
        };
        sky.set_sun_position(
            std::f32::consts::PI * (0.4983 - 0.5),
            2.0 * std::f32::consts::PI * (0.1979 - 0.5),
            400000.0,
        );
        sky
    }
}

impl PhysicalSkyMaterial {
    /// inclination in [-pi/2, pi/2], azimuth in [-pi, pi]
    pub fn set_sun_position(&mut self, inclination: f32, azimuth: f32, distance: f32) {
        self.sun_position.x = distance * azimuth.cos();
        self.sun_position.y = distance * azimuth.sin() * inclination.sin();
        self.sun_position.z = distance * azimuth.sin() * inclination.cos();
    }
}

pub fn setup(mut render_graph: ResMut<RenderGraph>) {
    // Add an AssetRenderResourcesNode to our Render Graph. This will bind
    // PhysicalSkyMaterial resources to our shader
    render_graph.add_system_node(
        PHYSICAL_SKY_RENDER_NODE,
        AssetRenderResourcesNode::<PhysicalSkyMaterial>::new(false),
    );

    // Add a Render Graph edge connecting our new PHYSICAL_SKY_RENDER_NODE node
    // to the main pass node. This ensures PHYSICAL_SKY_RENDER_NODE runs before
    // the main pass.
    render_graph
        .add_node_edge(PHYSICAL_SKY_RENDER_NODE, base::node::MAIN_PASS)
        .unwrap();
}
