use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{renderer::RenderResources, shader::ShaderDefs},
};

#[derive(RenderResources, TypeUuid, ShaderDefs)]
#[uuid = "84bde499-ec2f-4c23-a3cd-79c616cc8cf7"]
pub struct FadeUniform {
    pub duration: f32,
    pub remaining: f32,
    #[render_resources(ignore)]
    #[shader_def]
    pub fade_in: bool,
}

impl Default for FadeUniform {
    fn default() -> Self {
        Self {
            duration: 1.0,
            remaining: 1.0,
            fade_in: true,
        }
    }
}

pub fn mesh_fade_update_system(time: Res<Time>, mut fades: Query<&mut FadeUniform>) {
    for mut fade in fades.iter_mut() {
        fade.remaining = (fade.remaining - time.delta_seconds()).clamp(0.0, fade.duration);
    }
}
