use bevy::{
    core::Byteable,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::{RenderResource, RenderResources},
        shader::ShaderDefs,
    },
    transform::TransformSystem,
};

pub mod solar_position;

pub use chrono::prelude::*;
pub use solar_position::*;

pub const PHYSICAL_SKY_SETUP_SYSTEM: &str = "physical_sky_setup";
pub const PHYSICAL_SKY_PASS_TIME_SYSTEM: &str = "physical_sky_pass_time";
pub const PHYSICAL_SKY_TRACK_CAMERA_SYSTEM: &str = "physical_sky_track_camera";
pub const PHYSICAL_SKY_RENDER_NODE: &str = "physical_sky";
pub const PHYSICAL_SKY_VERTEX_SHADER: &str = include_str!("../assets/shaders/physical_sky.vert");
pub const PHYSICAL_SKY_FRAGMENT_SHADER: &str = include_str!("../assets/shaders/physical_sky.frag");

const SUN_DISTANCE: f32 = 400000.0;

pub struct PhysicalSkyPlugin;

impl Plugin for PhysicalSkyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<PhysicalSkyMaterial>()
            .add_startup_system(setup.system().label(PHYSICAL_SKY_SETUP_SYSTEM))
            .add_startup_system(pass_time.system())
            .add_system(pass_time.system().label(PHYSICAL_SKY_PASS_TIME_SYSTEM))
            .add_system_to_stage(
                CoreStage::PostUpdate,
                track_camera
                    .system()
                    .label(PHYSICAL_SKY_TRACK_CAMERA_SYSTEM)
                    .after(TransformSystem::ParentUpdate),
            );
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
    #[render_resources(ignore)]
    pub update_sun_position: bool,
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
            update_sun_position: false,
        };
        sky.set_sun_position(
            std::f32::consts::PI * (0.4983 - 0.5),
            2.0 * std::f32::consts::PI * (0.1979 - 0.5),
            SUN_DISTANCE,
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

    pub fn stellar_dawn(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(6.8e-7, 5.5e-7, 4.5e-7, 0.0),
            depolarization_factor: 0.067,
            luminance: 1.0,
            mie_coefficient: 0.00335,
            mie_directional_g: 0.787,
            mie_v: 4.012,
            mie_zenith_length: 500.0,
            num_molecules: 2.542e25,
            rayleigh_zenith_length: 615.0,
            rayleigh: 1.00,
            refractive_index: 1.000317,
            sun_angular_diameter_degrees: 0.00758,
            sun_intensity_factor: 1111.0,
            sun_intensity_falloff_steepness: 0.98,
            tonemap_weighting: 9.50,
            turbidity: 1.25,
            update_sun_position,
            ..Default::default()
        }
    }

    pub fn red_sunset(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(6.8e-7, 5.5e-7, 4.5e-7, 0.0),
            turbidity: 4.7,
            rayleigh: 2.28,
            mie_coefficient: 0.005,
            mie_directional_g: 0.82,
            luminance: 1.00,
            refractive_index: 1.00029,
            num_molecules: 2.542e25,
            depolarization_factor: 0.02,
            rayleigh_zenith_length: 8400.0,
            mie_v: 3.936,
            mie_zenith_length: 34000.0,
            sun_intensity_factor: 1000.0,
            sun_intensity_falloff_steepness: 1.5,
            sun_angular_diameter_degrees: 0.00933,
            tonemap_weighting: 9.50,
            update_sun_position,
            ..Default::default()
        }
    }

    pub fn alien_day(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(6.8e-7, 5.5e-7, 4.5e-7, 0.0),
            turbidity: 12.575,
            rayleigh: 5.75,
            mie_coefficient: 0.0074,
            mie_directional_g: 0.468,
            luminance: 1.00,
            refractive_index: 1.000128,
            num_molecules: 2.542e25,
            depolarization_factor: 0.137,
            rayleigh_zenith_length: 3795.0,
            mie_v: 4.007,
            mie_zenith_length: 7100.0,
            sun_intensity_factor: 1024.0,
            sun_intensity_falloff_steepness: 1.4,
            sun_angular_diameter_degrees: 0.006,
            tonemap_weighting: 9.50,
            update_sun_position,
            ..Default::default()
        }
    }

    pub fn blue_dusk(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(6.8e-7, 5.5e-7, 4.5e-7, 0.0),
            turbidity: 2.5,
            rayleigh: 2.295,
            mie_coefficient: 0.011475,
            mie_directional_g: 0.814,
            luminance: 1.00,
            refractive_index: 1.000262,
            num_molecules: 2.542e25,
            depolarization_factor: 0.095,
            rayleigh_zenith_length: 540.0,
            mie_v: 3.979,
            mie_zenith_length: 1000.0,
            sun_intensity_factor: 1151.0,
            sun_intensity_falloff_steepness: 1.22,
            sun_angular_diameter_degrees: 0.00639,
            tonemap_weighting: 9.50,
            update_sun_position,
            ..Default::default()
        }
    }

    pub fn purple_dusk(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(7.5e-7, 4.5e-7, 5.1e-7, 0.0),
            turbidity: 3.6,
            rayleigh: 2.26,
            mie_coefficient: 0.005,
            mie_directional_g: 0.822,
            luminance: 1.00,
            refractive_index: 1.000294,
            num_molecules: 2.542e25,
            depolarization_factor: 0.068,
            rayleigh_zenith_length: 12045.0,
            mie_v: 3.976,
            mie_zenith_length: 34000.0,
            sun_intensity_factor: 1631.0,
            sun_intensity_falloff_steepness: 1.5,
            sun_angular_diameter_degrees: 0.00933,
            tonemap_weighting: 9.50,
            update_sun_position,
            ..Default::default()
        }
    }

    pub fn blood_sky(update_sun_position: bool) -> Self {
        Self {
            mie_k_coefficient: Vec4::new(0.686, 0.678, 0.666, 0.0),
            primaries: Vec4::new(7.929e-7, 3.766e-7, 3.172e-7, 0.0),
            turbidity: 4.75,
            rayleigh: 6.77,
            mie_coefficient: 0.0191,
            mie_directional_g: 0.793,
            luminance: 1.1735,
            refractive_index: 1.000633,
            num_molecules: 2.542e25,
            depolarization_factor: 0.01,
            rayleigh_zenith_length: 1425.0,
            mie_v: 4.042,
            mie_zenith_length: 1600.0,
            sun_intensity_factor: 2069.0,
            sun_intensity_falloff_steepness: 2.26,
            sun_angular_diameter_degrees: 0.01487,
            tonemap_weighting: 9.50,
            update_sun_position,
            ..Default::default()
        }
    }
}

pub struct PhysicalSkyCameraTag;

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

pub fn track_camera(
    transforms: QuerySet<(
        Query<&GlobalTransform, With<PhysicalSkyCameraTag>>,
        Query<&mut GlobalTransform, With<Handle<PhysicalSkyMaterial>>>,
    )>,
) {
    let mut cam_temp = transforms.q0().iter();
    if let Some(camera_transform) = cam_temp.next() {
        transforms
            .q1()
            .for_each_mut(|mut mesh_transform| *mesh_transform = *camera_transform);
    }
}

pub fn pass_time(
    time: Res<Time>,
    mut solar_position: ResMut<SolarPosition>,
    query: Query<&Handle<PhysicalSkyMaterial>>,
    mut materials: ResMut<Assets<PhysicalSkyMaterial>>,
) {
    solar_position.tick(time.delta_seconds_f64());

    let (azimuth, inclination) = solar_position.get_azimuth_inclination();
    let (azimuth_radians, inclination_radians) = (
        (azimuth.to_radians() - std::f64::consts::PI) as f32,
        inclination.to_radians() as f32,
    );

    for handle in query.iter() {
        let material = materials.get_mut(handle).unwrap();
        if material.update_sun_position {
            material.set_sun_position(inclination_radians, azimuth_radians, SUN_DISTANCE);
        }
    }
}
