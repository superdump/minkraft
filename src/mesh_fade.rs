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
    pub delay: f32,
    #[render_resources(ignore)]
    #[shader_def]
    pub fade_in: bool,
}

impl Default for FadeUniform {
    fn default() -> Self {
        FADE_IN
    }
}

const FADE_DURATION: f32 = 0.25;
pub const FADE_IN: FadeUniform = FadeUniform {
    duration: FADE_DURATION,
    remaining: FADE_DURATION,
    delay: 0.0,
    fade_in: true,
};
pub const FADE_OUT: FadeUniform = FadeUniform {
    duration: FADE_DURATION,
    remaining: FADE_DURATION,
    delay: 0.0,
    fade_in: false,
};

pub fn mesh_fade_update_system(time: Res<Time>, mut fades: Query<&mut FadeUniform>) {
    for mut fade in fades.iter_mut() {
        let mut dt = time.delta_seconds();
        if fade.delay > 0.0 {
            if fade.delay > dt {
                fade.delay -= dt;
                continue;
            } else {
                dt -= fade.delay;
                fade.delay = 0.0;
            }
        }
        fade.remaining = (fade.remaining - dt).clamp(0.0, fade.duration);
    }
}
