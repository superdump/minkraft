use bevy::prelude::*;
use noise::*;

#[derive(Default)]
struct GenerateResource {
    pub noise: RidgedMulti,
}

pub struct GeneratePlugin;

impl Plugin for GeneratePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_resource::<GenerateResource>(GenerateResource {
                noise: RidgedMulti::new()
                    .set_seed(1234)
                    .set_frequency(0.01)
                    .set_octaves(5),
            })
            .add_startup_system(generate.system());
    }
}

fn generate(
    mut commands: Commands,
    noise: Res<GenerateResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mesh = meshes.add(Mesh::from(shape::Cube::default()));
    let material = materials.add(Color::GREEN.into());
    let yscale = 5.0f64;
    let n = 200;
    for z in 0..n {
        for x in 0..n {
            commands
                .spawn(PbrComponents {
                    translation: Vec3::new(
                        x as f32,
                        (noise.noise.get([x as f64, z as f64]) * yscale).round() as f32,
                        z as f32,
                    ).into(),
                    mesh,
                    material,
                    ..Default::default()
                });
        }
    }
}
