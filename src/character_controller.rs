use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_rapier3d::{na, physics::RigidBodyHandleComponent, rapier::dynamics::RigidBodySet};

// The structure of the Character is:
// * body
//   * PbrComponents model/mesh - here so that scale transform doesn't affect children
//   * a transform to offset for the head
//     * a transform to offset for third person view and the camera
//
// the body is created with a GlobalTransform, Transform (set the initial position), and kinematic
// RigidBody and Collider components for the physical presence of the body
//
// the first child entity of the body is a PbrComponents bundle for the model (set the scale of the model here)
//
// the second child entity of the body is a Transform (and GlobalTransform) to offset the head. This gives a
// fixed point of reference for where the head/eyes are. This allows either a first person or third person
// viewer to rotate around this point
//
// the final entity is a child of the head. It has a Transform (and GlobalTransform) and camera and it acts
// like a camera boom arm attached to the head. This should enable use as a follow camera or first person
// (perhaps no PbrComponents in that case)
//
// when translating, the position of the rigid body should be manipulated
// when yawing (rotating about the y axis), the orientation of the rigid body should be manipulated
// when pitching (rotating about the right axis relative to the character), the orientation of the
//   head should be manipulated
// when zooming in and out or changing the focal point, the translation and orientation of the camera
//   boom should be manipulated
//
// further work would be to have some lag/smoothing of the camera motion relative to the body
// this would separate the yawing of the look direction to also affect the head with a tendency
// toward the forward direction of the body. this is for controller styles more common to third-person
// where up/down/left/right turn the character toward the direction of movement and the camera does not
// directly follow after such that running in a zig-zag may keep the camera in the same position.
// however, if using the right-stick or mouse, you can take control of the camera orientation to be
// absolute.

pub struct CharacterController {
    pub enabled: bool,
    pub jump_velocity: f32,
    pub fall_acceleration_factor: f32,
    pub mouse_sensitivity: f32,
    pub max_speed: f32,
    pub key_toggle: KeyCode,
    pub key_forward: KeyCode,
    pub key_backward: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_jump: KeyCode,
    pub velocity: Vec3,
    pub friction: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub delta_yaw: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        CharacterController {
            enabled: true,
            jump_velocity: 10.0,
            fall_acceleration_factor: 2.0,
            mouse_sensitivity: 1.0,
            max_speed: 10.0,
            key_toggle: KeyCode::M,
            key_forward: KeyCode::W,
            key_backward: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_jump: KeyCode::Space,
            velocity: Vec3::zero(),
            friction: Vec3::new(10.0, 0.0, 10.0),
            yaw: 0.0,
            pitch: 0.0,
            delta_yaw: 0.0,
        }
    }
}

pub struct CharacterControllerBodyTag;
pub struct CharacterControllerHeadTag;
pub struct CharacterControllerCameraTag;

fn character_controller_toggle(
    keyboard_input: Res<Input<KeyCode>>,
    mut options: Mut<CharacterController>,
) {
    if keyboard_input.just_pressed(options.key_toggle) {
        options.enabled = !options.enabled;
    }
}

// *** NOTE ***
// The code contained here is copied from:
//   https://github.com/mcpar-land/bevy_fly_camera
// which is also MIT-licensed
// Copyright 2020 mcpar-land
// Copyright 2020 Robert Swain <robert.swain@gmail.com>
// *** START OF COPIED AND MODIFIED CODE ***
fn forward_vector(rotation: &Quat) -> Vec3 {
    rotation.mul_vec3(Vec3::unit_z()).normalize()
}

fn forward_walk_vector(rotation: &Quat) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x(), 0.0, f.z()).normalize();
    f_flattened
}

fn strafe_vector(rotation: &Quat) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Quat::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}

fn movement_axis(input: &Res<Input<KeyCode>>, plus: KeyCode, minus: KeyCode) -> f32 {
    let mut axis = 0.0;
    if input.pressed(plus) {
        axis += 1.0;
    }
    if input.pressed(minus) {
        axis -= 1.0;
    }
    axis
}

fn character_controller_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<With<CharacterControllerBodyTag, (&mut CharacterController, &Transform)>>,
) {
    for (mut options, transform) in &mut query.iter() {
        let (axis_h, axis_v, axis_float) = if options.enabled {
            (
                movement_axis(&keyboard_input, options.key_right, options.key_left),
                movement_axis(&keyboard_input, options.key_backward, options.key_forward),
                if keyboard_input.pressed(options.key_jump) {
                    1.0
                } else {
                    0.0
                },
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let rotation = transform.rotation();
        let accel: Vec3 = (strafe_vector(&rotation) * axis_h)
            + (forward_walk_vector(&rotation) * axis_v)
            + (Vec3::unit_y() * axis_float);
        // - 9.81 * Vec3::unit_y();
        let accel: Vec3 = if accel.length() != 0.0 {
            accel.normalize() * options.max_speed
        } else {
            Vec3::zero()
        };

        let friction: Vec3 = if options.velocity.length() != 0.0 {
            options.velocity.normalize() * -1.0 * options.friction
        } else {
            Vec3::zero()
        };

        options.velocity += accel * time.delta_seconds;

        // clamp within max speed
        if options.velocity.length() > options.max_speed {
            options.velocity = options.velocity.normalize() * options.max_speed;
        }

        let delta_friction = friction * time.delta_seconds;

        options.velocity = if (options.velocity + delta_friction).sign() != options.velocity.sign()
        {
            Vec3::zero()
        } else {
            options.velocity + delta_friction
        };
    }
}

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}

fn character_controller_look(
    time: Res<Time>,
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    mut query_body: Query<With<CharacterControllerBodyTag, &mut CharacterController>>,
    mut query_head: Query<With<CharacterControllerHeadTag, &mut Transform>>,
) {
    let mut delta: Vec2 = Vec2::zero();
    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        delta += event.delta;
    }

    let mut query_head_temp = query_head.iter();
    let mut head_transform = query_head_temp.iter().next().unwrap();
    for mut options in &mut query_body.iter() {
        if delta == Vec2::zero() {
            options.delta_yaw = 0.0;
        } else {
            if !options.enabled {
                continue;
            }

            let delta_yaw = -delta.x() * options.mouse_sensitivity * time.delta_seconds;
            options.yaw += delta_yaw;
            options.delta_yaw = delta_yaw;
            options.pitch -= delta.y() * options.mouse_sensitivity * time.delta_seconds;

            if options.pitch > 89.9 {
                options.pitch = 89.9;
            }
            if options.pitch < -89.9 {
                options.pitch = -89.9;
            }
        }

        head_transform.set_rotation(Quat::from_rotation_ypr(
            0.0,
            options.pitch.to_radians(),
            0.0,
        ));
    }
}

fn character_controller_set_next(
    mut bodies: ResMut<RigidBodySet>,
    mut query_body: Query<
        With<CharacterControllerBodyTag, (&CharacterController, &RigidBodyHandleComponent)>,
    >,
) {
    for (options, body_handle) in &mut query_body.iter() {
        if let Some(mut body) = bodies.get_mut(body_handle.handle()) {
            let mut isometry = body.position;
            isometry.append_translation_mut(&na::Translation3::new(
                options.velocity.x(),
                options.velocity.y(),
                options.velocity.z(),
            ));
            isometry.append_rotation_wrt_center_mut(&na::UnitQuaternion::from_axis_angle(
                &na::Vector3::y_axis(),
                options.delta_yaw.to_radians(),
            ));
            body.set_next_kinematic_position(isometry);
        }
    }
}

/**
Include this plugin to add the systems for the CharacterController bundle.

```no_run
fn main() {
    App::build().add_plugin(CharacterControllerPlugin);
}
```

**/

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<State>()
            .add_system(character_controller_toggle.system())
            .add_system(character_controller_movement.system())
            .add_system(character_controller_look.system())
            .add_system(character_controller_set_next.system());
    }
}
// *** END OF COPIED AND MODIFIED CODE ***
