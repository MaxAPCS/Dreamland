use bevy::{
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::tonemapping::Tonemapping,
    light::{
        Atmosphere, CascadeShadowConfigBuilder, DirectionalLightShadowMap,
        atmosphere::ScatteringMedium,
    },
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
    render::occlusion_culling::OcclusionCulling,
};

mod bindings;
mod compute;
mod quadtree;
mod world;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        recognize_pinch_gesture: false,
                        recognize_rotation_gesture: false,
                        recognize_doubletap_gesture: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "warn,bevy_pbr=error".to_string(), // dreamland=debug,
                    ..default()
                }),
        )
        .insert_resource(Time::<Fixed>::from_hz(24.))
        .add_plugins(FreeCameraPlugin)
        .add_plugins(compute::ComputePlugin)
        .add_plugins(world::WorldPlugin)
        .add_systems(Startup, (setup_world, setup_camera))
        .add_systems(FixedUpdate, animate_light_direction)
        .run();
}

fn setup_camera(mut commands: Commands, mut scattering_mediums: ResMut<Assets<ScatteringMedium>>) {
    let earth_medium = scattering_mediums.add(ScatteringMedium::default());
    commands.spawn(Atmosphere {
        inner_radius: 128000.,
        outer_radius: 256000.,
        ground_albedo: Vec3::new(0., 120., 23.) * 0.5 / 255. + Vec3::splat(0.3),
        medium: earth_medium,
    });
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3 {
            x: 0.,
            y: 160.,
            z: 0.,
        }),
        FreeCamera {
            sensitivity: 0.24,
            friction: 16.0,
            run_speed: 64.,
            mouse_key_cursor_grab: MouseButton::Left,
            keyboard_key_toggle_cursor_grab: KeyCode::Escape,
            key_up: KeyCode::Space,
            key_down: KeyCode::ShiftLeft,
            key_run: KeyCode::KeyC,
            ..default()
        },
        DistanceFog {
            falloff: FogFalloff::from_visibility_squared(1024.0),
            ..default()
        },
        AtmosphereSettings::default(),
        Exposure::SUNLIGHT,
        Bloom::NATURAL,
        Tonemapping::Reinhard,
        OcclusionCulling,
    ));
}

fn setup_world(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::DIRECT_SUNLIGHT,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.6,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: world::LEVELS[2] as f32,
            first_cascade_far_bound: world::LEVELS[0] as f32,
            ..default()
        }
        .build(),
    ));
    commands.insert_resource(DirectionalLightShadowMap { size: 1024 });
}

fn animate_light_direction(
    time: Res<Time>,
    mut light: Single<&mut Transform, With<DirectionalLight>>,
) {
    const DAY_LENGTH: f32 = 64.;

    // 24-hour cycle - 0 midnight, 0.5 dawn, 1 noon
    let t = (time.elapsed_secs() % DAY_LENGTH) / DAY_LENGTH;
    let time_of_day = 2.75 * t - 4.25 * t * t + 2.5 * t.powi(3);
    let sun_angle = time_of_day * 2.0 * std::f32::consts::PI;

    // Calculate sun position (simplified circular path)
    let sun_elevation = (sun_angle - std::f32::consts::PI).sin();
    let sun_azimuth = sun_angle.cos();

    // For DirectionalLight, we point the light's negative Z axis toward the sun
    // So we need to rotate the light to face the opposite direction (from sun to earth)
    light.rotation = Quat::from_euler(
        EulerRot::XYZ,
        -sun_elevation, // tilt down for negative elevation
        sun_azimuth,    // rotate around Y axis
        0.0,
    );
}
