
use bevy::{
    math::{Vec2Swizzles, Vec4Swizzles, vec2, vec3, vec4},
    prelude::*,
};

fn permute_3(x: Vec3) -> Vec3 {
    (((x * 34.) + 1.) * x) % Vec3::splat(289.)
}
// MIT License. © Ian McEwan, Stefan Gustavson, Munrocket, Johan Helsing
/// Simplex noise in two dimensions
pub fn simplex_noise_2d_seeded(v: Vec2, seed: f32) -> f32 {
    const C: Vec4 = vec4(
        0.211_324_87,  // (3.0 - sqrt(3.0)) / 6.0
        0.366_025_42,  // 0.5 * (sqrt(3.0) - 1.0)
        -0.577_350_26, // -1.0 + 2.0 * C.x
        1. / 41.,
    );

    // first corner
    let mut i: Vec2 = (v + Vec2::dot(v, C.yy())).floor();
    let x0 = v - i + Vec2::dot(i, C.xx());

    // other corners
    let i1: Vec2 = if x0.x > x0.y {
        vec2(1., 0.)
    } else {
        vec2(0., 1.)
    };
    let x12: Vec4 = x0.xyxy() + C.xxzz() - vec4(i1.x, i1.y, 0., 0.);

    // permutations
    i %= Vec2::splat(289.);

    let mut p = permute_3(permute_3(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    p = permute_3(p + Vec3::splat(seed));

    let mut m = Vec3::max(
        0.5 - vec3(
            Vec2::dot(x0, x0),
            Vec2::dot(x12.xy(), x12.xy()),
            Vec2::dot(x12.zw(), x12.zw()),
        ),
        Vec3::splat(0.),
    );
    m *= m;
    m *= m;

    // gradients: 41 points uniformly over a line, mapped onto a diamond
    // the ring size, 17*17 = 289, is close to a multiple of 41 (41*7 = 287)
    let x = 2. * (p * C.www()).fract() - 1.;
    let h = x.abs() - 0.5;
    let ox = (x + 0.5).floor();
    let a0 = x - ox;

    // normalize gradients implicitly by scaling m
    // approximation of: m *= inversesqrt(a0 * a0 + h * h);
    m *= 1.792_842_9 - 0.853_734_73 * (a0 * a0 + h * h);
    let g = vec3(
        a0.x * x0.x + h.x * x0.y,
        a0.y * x12.x + h.y * x12.y,
        a0.z * x12.z + h.z * x12.w,
    );

    // compute final noise value at P
    130. * Vec3::dot(m, g)
}

/// Fractional brownian motion (fbm) based on seeded 2d simplex noise
pub fn fbm_simplex_2d_seeded(
    pos: Vec2,
    octaves: usize,
    lacunarity: f32,
    gain: f32,
    seed: f32,
) -> f32 {
    let mut sum = 0.;
    let mut amplitude = 1.;
    let mut frequency = 0.01;

    for _ in 0..octaves {
        sum += simplex_noise_2d_seeded(pos * frequency, seed) * amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    sum
}