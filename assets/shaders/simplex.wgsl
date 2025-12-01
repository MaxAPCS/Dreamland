// Authors: Stefan Gustavson (stefan.gustavson@gmail.com)
// and Ian McEwan (ijm567@gmail.com)
// Version 2022-02-28, published under the MIT license (see below)
//
// Copyright (c) 2021-2022 Stefan Gustavson and Ian McEwan.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

fn mod289v3f(x: vec3<f32>) -> vec3<f32> {
	return x - floor(x / 289.0) * 289.0;
}

fn srdnoise2(x: vec2<f32>, alpha: f32) -> vec3<f32> {
	var uv: vec2<f32>;
	var f0: vec2<f32>;
	var i0: vec2<f32>;
	var i1: vec2<f32>;
	var i2: vec2<f32>;
	var o1: vec2<f32>;
	var v0: vec2<f32>;
	var v1: vec2<f32>;
	var v2: vec2<f32>;
	var x0: vec2<f32>;
	var x1: vec2<f32>;
	var x2: vec2<f32>;
	
	uv = vec2<f32>(x.x+x.y*0.5, x.y);
	i0 = floor(uv);  // modf() is not a modulo operation!
	f0 = uv - i0;
	o1 = select(vec2<f32>(0.0,1.0), vec2<f32>(1.0, 0.0), f0.x > f0.y);
	i1 = i0 + o1;
	i2 = i0 + vec2<f32>(1.0, 1.0);
	v0 = vec2<f32>(i0.x - i0.y*0.5, i0.y);
	v1 = vec2<f32>(v0.x + o1.x - o1.y*0.5, v0.y + o1.y);
	v2 = vec2<f32>(v0.x + 0.5, v0.y + 1.0);
	x0 = x - v0;
	x1 = x - v1;
	x2 = x - v2;

	var iu: vec3<f32>;
	var iv: vec3<f32>;

	iu = vec3<f32>(i0.x, i1.x, i2.x);
	iv = vec3<f32>(i0.y, i1.y, i2.y);

	var hash: vec3<f32>;
	var psi: vec3<f32>;
	var gx: vec3<f32>;
	var gy: vec3<f32>;
	var g0: vec2<f32>;
	var g1: vec2<f32>;
	var g2: vec2<f32>;

	hash = mod289v3f(iu);
	hash = mod289v3f((hash*51.0 + 2.0)*hash + iv);
	hash = mod289v3f((hash*34.0 + 10.0)*hash);
	psi = hash*0.07482 + alpha;
	gx = cos(psi);
	gy = sin(psi);
	g0 = vec2<f32>(gx.x, gy.x);
	g1 = vec2<f32>(gx.y, gy.y);
	g2 = vec2<f32>(gx.z, gy.z);

	var w: vec3<f32>;
	var w2: vec3<f32>;
	var w4: vec3<f32>;
	var gdotx: vec3<f32>;
	var n: f32;

	w = 0.8 - vec3<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2));
	w = max(w, vec3<f32>(0.0, 0.0, 0.0));
	w2 = w*w;
	w4 = w2*w2;
	gdotx = vec3<f32>(dot(g0, x0), dot(g1, x1), dot(g2, x2));
	n = 10.9*dot(w4, gdotx);

	var w3: vec3<f32>;
	var dw: vec3<f32>;
	var dn0: vec2<f32>;
	var dn1: vec2<f32>;
	var dn2: vec2<f32>;
	var g: vec2<f32>;
	w3 = w2*w;
	dw = -8.0*w3*gdotx;
	dn0 = w4.x*g0 + dw.x*x0;
	dn1 = w4.y*g1 + dw.y*x1;
	dn2 = w4.z*g2 + dw.z*x2;
	g = 10.9*(dn0 + dn1 + dn2);

	return vec3(n, g);
}

const WORKGROUPX = 8u;
const WORKGROUPY = 1u;
const WORKGROUPZ = 8u;
const WORKGROUP_SIZE = WORKGROUPX * WORKGROUPY * WORKGROUPZ;

struct FractalNoiseConfig {
	seed: f32,
	octaves: u32,
	lacunarity: f32,
	gain: f32,
	erosion: f32,
	scale: f32,
	offset: f32
}

struct FractalNoiseBounds {
	x: f32, // Minimum X Coordinate
	z: f32, // Minimum Z Coordinate
	xstep: f32, // X Increment Per main()
	zstep: f32  // Z Increment per main()
	// Maximum X Coordinate = x + xstep * num_workgroups.x * workgroup_size.x
	//                      = x + xstep * samples.x
}

struct PointAndNormal {
	point: f32,
	normal: array<f32, 3>
}

@group(0) @binding(0) var<uniform> config: FractalNoiseConfig;

@group(0) @binding(1) var<uniform> bounds: FractalNoiseBounds;

@group(0) @binding(2) var<storage, read_write> output: array<PointAndNormal>;

/// Fractional brownian motion (fbm) based on seeded 2d simplex noise
@compute @workgroup_size(WORKGROUPX,WORKGROUPY,WORKGROUPZ)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let pos = vec2(bounds.x + bounds.xstep * f32(global_invocation_id.x), bounds.z + bounds.zstep * f32(global_invocation_id.z)) / 96.;

    var sum = 0.;
    var amplitude = 1.;
    var frequency = 1.;
    var gradient = vec2<f32>();

    for (var i = 0u; i < config.octaves; i++) {
        let noise = srdnoise2(pos * frequency, config.seed) * amplitude / (1. + config.erosion * dot(gradient, gradient));
        gradient += noise.yz;
        sum += noise.x;
        frequency *= config.lacunarity;
        amplitude *= config.gain;
    }

		let norm = normalize(vec3(-gradient.xy * config.scale, 1));

    output[
        global_invocation_id.x + global_invocation_id.z * (num_workgroups.x * WORKGROUPX)
    ] = PointAndNormal(sum * config.scale + config.offset, array(norm.x, norm.z, norm.y)); // transform coordinate systems
}
