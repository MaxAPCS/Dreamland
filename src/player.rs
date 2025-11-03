use avian3d::prelude::*;
use bevy::math::ops::{cos, sin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_tnua::TnuaUserControlsSystems;
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};
use std::f32::consts::FRAC_PI_2;

pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(Startup, initialize)
        .add_systems(FixedUpdate, update_controls.in_set(TnuaUserControlsSystems))
        .add_systems(
            PostUpdate,
            update_camera
                .after(avian3d::prelude::PhysicsSystems::Writeback)
                .before(TransformSystems::Propagate),
        );
    }
}

#[derive(Component)]
pub(crate) struct Player;

#[derive(Resource)]
struct PastMouseLocation {
    location: Option<Vec2>,
}

impl PastMouseLocation {
    fn delta(&mut self, current: &Vec2) -> Vec2 {
        if let Some(past) = self.location {
            self.location = Some(*current);
            *current - past
        } else {
            self.location = Some(*current);
            Vec2::ZERO
        }
    }
}

#[derive(Resource)]
struct PlayerRotation {
    // All In Radians
    yaw: f32,
    pitch: f32,
}

impl PlayerRotation {
    fn to_dir3_flat(&self) -> Dir3 {
        // Yaw 0 = [0, 0, -1]
        // Yaw 90 = [-1, 0, 0]
        Dir3::from_xyz_unchecked(-sin(self.yaw), 0.0, -cos(self.yaw))
    }

    fn to_quat(&self) -> Quat {
        Quat::from_euler(EulerRot::default(), self.yaw, self.pitch, 0.0)
    }
}

fn initialize(mut commands: Commands) {
    commands.insert_resource(PlayerRotation {
        yaw: 0.0,
        pitch: 0.0,
    });

    commands.insert_resource(PastMouseLocation { location: None });

    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        TnuaController::default(),
        // A sensor shape is not strictly necessary, but without it we'll get weird results.
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
    ));
}

fn update_controls(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,

    mut my_playerrot: ResMut<PlayerRotation>,
    mut my_pastmouse: ResMut<PastMouseLocation>,

    mut cursor_options: Single<&mut CursorOptions>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,

    mut query: Query<&mut TnuaController>,
) {
    let Ok(mut controller) = query.single_mut() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Confined;
    }

    if keyboard.pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }

    if cursor_options.grab_mode != CursorGrabMode::None {
        for event in mouse_motion.read() {
            let delta = my_pastmouse.delta(&event.delta);

            my_playerrot.yaw -= delta.x / 100.0;
            my_playerrot.pitch -= delta.y / 200.0;
        }
        my_playerrot.pitch = my_playerrot.pitch.clamp(-FRAC_PI_2, FRAC_PI_2);
    }

    let forward = my_playerrot.to_dir3_flat();

    let mut direction = Vec3::ZERO;
    if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        direction += forward.as_vec3();
    }
    if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        direction -= forward.as_vec3();
    }
    if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        direction -= forward.cross(Vec3::Y);
    }
    if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        direction += forward.cross(Vec3::Y);
    }
    direction = direction.normalize_or_zero();
    if keyboard.any_pressed([KeyCode::ShiftLeft]) {
        direction *= 1.5;
    }

    controller.basis(TnuaBuiltinWalk {
        desired_forward: Some(forward),
        desired_velocity: direction * 10.0,
        // The `float_height` must be greater (even if by little) from the distance between the
        // character's center and the lowest point of its collider.
        float_height: 1.5,
        ..Default::default()
    });

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 2.0,
            ..Default::default()
        });
    }
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    rotation: Res<PlayerRotation>,
    player_query: Query<&Transform, (With<Player>, Changed<Transform>, Without<Camera3d>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    camera_transform.translation = player_transform.translation;
    camera_transform.rotation = rotation.to_quat();
}
