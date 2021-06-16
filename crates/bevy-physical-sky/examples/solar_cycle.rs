use bevy::{
    input::system::exit_on_esc_system,
    prelude::*,
    render::{
        pipeline::{FrontFace, PipelineDescriptor, RenderPipeline},
        shader::{ShaderStage, ShaderStages},
    },
};
use bevy_physical_sky::{
    PhysicalSkyCameraTag, PhysicalSkyMaterial, PhysicalSkyPlugin, SolarPosition, TimeZone, Utc,
    PHYSICAL_SKY_FRAGMENT_SHADER, PHYSICAL_SKY_SETUP_SYSTEM, PHYSICAL_SKY_VERTEX_SHADER,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(exit_on_esc_system.system())
        .insert_resource(SolarPosition {
            // Stockholm
            latitude: 59.33258,
            longitude: 18.0649,
            // one day per 30 seconds of real time
            simulation_seconds_per_second: 24.0 * 60.0 * 60.0 / 30.0,
            now: Utc.ymd(2021, 03, 01).and_hms(7, 0, 0),
            ..Default::default()
        })
        .add_plugin(PhysicalSkyPlugin)
        .add_startup_system(setup.system().after(PHYSICAL_SKY_SETUP_SYSTEM))
        .run();
}

fn setup(
    mut shaders: ResMut<Assets<Shader>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut sky_materials: ResMut<Assets<PhysicalSkyMaterial>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create a new shader pipeline
    let mut pipeline_descriptor = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            PHYSICAL_SKY_VERTEX_SHADER,
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            PHYSICAL_SKY_FRAGMENT_SHADER,
        ))),
    });
    // Reverse the winding so we can see the faces from the inside
    pipeline_descriptor.primitive.front_face = FrontFace::Cw;
    let pipeline = pipelines.add(pipeline_descriptor);

    // Create a new material
    let material = sky_materials.add(PhysicalSkyMaterial {
        update_sun_position: 1,
        ..Default::default()
    });

    // plane
    commands
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 15.0 })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(pipeline)]),
            transform: Transform::default(),
            ..Default::default()
        })
        .insert(material);

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0)
                .looking_at(Vec3::new(1.0, 0.0, 1.0), Vec3::Y),
            ..Default::default()
        })
        .insert(PhysicalSkyCameraTag);
}
