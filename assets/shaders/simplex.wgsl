const WORKGROUP_SQRT = 8u;

struct FractalNoiseConfig {
    seed: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
}

fn permute_3_(x: vec3<f32>) -> vec3<f32> {
    return (((x * 34.) + 1.) * x) % vec3(289.);
}

fn simplex_noise_2d_seeded(v: vec2<f32>, seed: f32) -> f32 {
    let C = vec4(
        0.211324865405187, // (3.0 - sqrt(3.0)) / 6.0
        0.366025403784439, // 0.5 * (sqrt(3.0) - 1.0)
        -0.577350269189626, // -1.0 + 2.0 * C.x
        0.024390243902439 // 1.0 / 41.0
    );

    // first corner
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    // other corners
    var i1 = select(vec2(0., 1.), vec2(1., 0.), x0.x > x0.y);
    var x12 = x0.xyxy + C.xxzz - vec4(i1, 0., 0.);

    // permutations
    i = i % vec2(289.);

    var p = permute_3_(permute_3_(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    p = permute_3_(p + vec3(seed));
    var m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3(0.));
    m *= m;
    m *= m;

    // gradients: 41 points uniformly over a line, mapped onto a diamond
    // the ring size, 17*17 = 289, is close to a multiple of 41 (41*7 = 287)
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    // normalize gradients implicitly by scaling m
    // approximation of: m *= inversesqrt(a0 * a0 + h * h);
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));

    // compute final noise value at P
    let g = vec3(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}

@group(0) @binding(0)
var<uniform> config: FractalNoiseConfig;

@group(0) @binding(1) var<storage, read> positions: array<vec2<f32>, WORKGROUP_SQRT * WORKGROUP_SQRT>;
@group(0) @binding(2) var<storage, read_write> output: array<f32, WORKGROUP_SQRT * WORKGROUP_SQRT>;

/// Fractional brownian motion (fbm) based on seeded 2d simplex noise
@compute @workgroup_size(WORKGROUP_SQRT,WORKGROUP_SQRT,1)
fn main(
    @builtin(local_invocation_index) local_invocation_index : u32,
) {
    let pos = positions[local_invocation_index];

    var sum = 0.;
    var amplitude = 1.;
    var frequency = 1.;

    for (var i = 0u; i < config.octaves; i++) {
        sum += simplex_noise_2d_seeded(pos * frequency, seed) * amplitude;
        frequency *= config.lacunarity;
        amplitude *= config.gain;
    }

    output[local_invocation_index] = sum;
}
