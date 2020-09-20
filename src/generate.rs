use bevy::{
    prelude::*,
    render::{mesh::VertexAttribute, pipeline::PrimitiveTopology},
};
use noise::*;

#[derive(Default)]
struct GenerateResource {
    pub noise: RidgedMulti,
}

pub struct GeneratePlugin;

impl Plugin for GeneratePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource::<GenerateResource>(GenerateResource {
            noise: RidgedMulti::new()
                .set_seed(1234)
                .set_frequency(0.01)
                .set_octaves(5),
        })
        .add_startup_system(generate_mesh.system());
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
    let n = 100;
    for z in 0..n {
        for x in 0..n {
            commands.spawn(PbrComponents {
                transform: Transform::from_translation(Vec3::new(
                    x as f32,
                    (noise.noise.get([x as f64, z as f64]) * yscale).round() as f32,
                    z as f32,
                )),
                mesh,
                material,
                ..Default::default()
            });
        }
    }
}

fn index(n: usize, x: usize, y: usize, z: usize) -> usize {
    ((y * n) + z) * n + x
}

fn cell_bounds_check(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx < n * n * n => Some(idx),
        _ => None,
    }
}

fn cell_above(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx < n * n * (n - 1) => cell_bounds_check(n, idx + n * n),
        _ => None,
    }
}

fn cell_below(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx >= n * n => cell_bounds_check(n, idx - n * n),
        _ => None,
    }
}

fn cell_front(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx < n * (n * n - 1) => cell_bounds_check(n, idx + n),
        _ => None,
    }
}

fn cell_back(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx >= n => cell_bounds_check(n, idx - n),
        _ => None,
    }
}

fn cell_right(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx < n * n * n - 1 => cell_bounds_check(n, idx + 1),
        _ => None,
    }
}

fn cell_left(n: usize, idx: usize) -> Option<usize> {
    match idx {
        idx if idx >= 1 => cell_bounds_check(n, idx - 1),
        _ => None,
    }
}

fn create_face_above(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [0.0f32, 1.0f32, 0.0f32];
    // X, Y + 1, Z
    positions.push([x as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X, Y + 1, Z + 1
    positions.push([x as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X + 1, Y + 1, Z + 1
    positions.push([(x + 1) as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X + 1, Y + 1, Z
    positions.push([(x + 1) as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 1, i + 2, i, i + 2, i + 3];
    indices.append(&mut new_indices);
}

fn create_face_below(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [0.0f32, -1.0f32, 0.0f32];
    // X, Y, Z
    positions.push([x as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X, Y, Z + 1
    positions.push([x as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X + 1, Y, Z + 1
    positions.push([(x + 1) as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X + 1, Y, Z
    positions.push([(x + 1) as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 2, i + 1, i, i + 3, i + 2];
    indices.append(&mut new_indices);
}

fn create_face_front(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [0.0f32, 0.0f32, 1.0f32];
    // X, Y, Z + 1
    positions.push([x as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X, Y + 1, Z + 1
    positions.push([x as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X + 1, Y + 1, Z + 1
    positions.push([(x + 1) as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X + 1, Y, Z + 1
    positions.push([(x + 1) as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 2, i + 1, i, i + 3, i + 2];
    indices.append(&mut new_indices);
}

fn create_face_back(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [0.0f32, 0.0f32, -1.0f32];
    // X, Y, Z
    positions.push([x as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X, Y + 1, Z
    positions.push([x as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X + 1, Y + 1, Z
    positions.push([(x + 1) as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X + 1, Y, Z
    positions.push([(x + 1) as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 2, i + 1, i, i + 3, i + 2];
    indices.append(&mut new_indices);
}

fn create_face_right(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [1.0f32, 0.0f32, 0.0f32];
    // X + 1, Y, Z
    positions.push([(x + 1) as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X + 1, Y, Z + 1
    positions.push([(x + 1) as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X + 1, Y + 1, Z + 1
    positions.push([(x + 1) as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X + 1, Y + 1, Z
    positions.push([(x + 1) as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 2, i + 1, i, i + 3, i + 2];
    indices.append(&mut new_indices);
}

fn create_face_left(
    x: usize,
    y: usize,
    z: usize,
    i: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let normal = [-1.0f32, 0.0f32, 0.0f32];
    // X, Y, Z
    positions.push([x as f32, y as f32, z as f32]);
    normals.push(normal);
    uvs.push([0.0, 0.0]);
    // X, Y, Z + 1
    positions.push([x as f32, y as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([0.0, 1.0]);
    // X, Y + 1, Z + 1
    positions.push([x as f32, (y + 1) as f32, (z + 1) as f32]);
    normals.push(normal);
    uvs.push([1.0, 1.0]);
    // X, Y + 1, Z
    positions.push([x as f32, (y + 1) as f32, z as f32]);
    normals.push(normal);
    uvs.push([1.0, 0.0]);

    let mut new_indices = vec![i, i + 2, i + 1, i, i + 3, i + 2];
    indices.append(&mut new_indices);
}

fn generate_mesh(
    mut commands: Commands,
    noise: Res<GenerateResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Chunk generation
    let yoffset = 5.0f64;
    let yscale = 5.0f64;
    let n = 100;
    let mut cells = vec![false; n * n * n];
    for z in 0..n {
        for x in 0..n {
            let y = (noise.noise.get([x as f64, z as f64]) * yscale + yoffset).round() as usize;
            cells[index(n, x, y, z)] = true;
        }
    }

    // Mesh generation
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for y in 0..n {
        for z in 0..n {
            for x in 0..n {
                let idx = index(n, x, y, z);
                if !cells[idx] {
                    continue;
                }
                if let Some(neighbour) = cell_above(n, idx) {
                    if !cells[neighbour] {
                        // face above
                        create_face_above(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
                if let Some(neighbour) = cell_below(n, idx) {
                    if !cells[neighbour] {
                        // face below
                        create_face_below(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
                if let Some(neighbour) = cell_front(n, idx) {
                    if !cells[neighbour] {
                        // face front
                        create_face_front(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
                if let Some(neighbour) = cell_back(n, idx) {
                    if !cells[neighbour] {
                        // face back
                        create_face_back(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
                if let Some(neighbour) = cell_right(n, idx) {
                    if !cells[neighbour] {
                        // face right
                        create_face_right(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
                if let Some(neighbour) = cell_left(n, idx) {
                    if !cells[neighbour] {
                        // face left
                        create_face_left(
                            x,
                            y,
                            z,
                            i,
                            &mut positions,
                            &mut normals,
                            &mut uvs,
                            &mut indices,
                        );
                        i += 4;
                    }
                }
            }
        }
    }

    let mesh = meshes.add(Mesh {
        primitive_topology: PrimitiveTopology::TriangleList,
        attributes: vec![
            VertexAttribute::position(positions),
            VertexAttribute::normal(normals),
            VertexAttribute::uv(uvs),
        ],
        indices: Some(indices),
    });
    let material = materials.add(Color::GREEN.into());
    commands.spawn(PbrComponents {
        mesh,
        material,
        ..Default::default()
    });
}
