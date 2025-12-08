# Dreamland
**A 3D procedural world generator using Rust, Bevy, WASM, and WebGPU.**

Dreamland is an infinite world of procedural terrain that continuously generates around the user, rendering in high quality close by and tapering off when far away. Fractal Brownian Motion is applied to simplex noise to sample heights in parallel on the GPU, which are passed back to the CPU and turned into triangle strip meshes with smooth shading. These are dynamically generated and destroyed at variable detail levels by an efficient lazy quadtree algorithm, permitting nearly unlimited world size.
This project is designed firstly to be a personal foray into the subject, and perhaps alternatively as a flexible visualization of terrain generators for game developers, 3D artists, and others. This experience is designed to be usable and performant on the web, enabling greater accessibility.
Dreamland works on Firefox (with flags), Chromium-based browsers, and is untested on Safari.

## Usage
W/A/S/D: Movement
Shift: Sprint
Space: Jump
Mouse: Camera Direction

To bind the camera, click on the window once it's loaded, then press escape once. Pressing escape again will free your cursor.

## Building & Deploying
To build, ensure `wasm-bindgen-cli` v0.2.106 and `wasm-opt` are installed with `cargo install`, then run `./build.sh`.
To deploy, simply serve the `serve/` directory (e.g. with `python -m http.server`)

## Roadmap

As of now, the project has all of its core features, and I would consider it complete. Work will continue to add polish wherever possible.

### Complete
- CPU Noise Generation
- Physics Engine & Character Controller
- Procedural Atmosphere
- GPU Compute Noise Generation
- Quadtree Framework & Dynamic LODs
- Analytical Derivative Erosion Trick
- Macroscopic / Continental Noise

### Planned
- Biomes / Parameterized Noise
- Truly Infinite Generation
- Mobile Device Accelerometer Camera Control
- Procedural Foliage
- Procedural Flat Cloud Fragment Shader
- Terrain Coloring Fragment Shader
- Curved World Vertex Shader

## Acknowledgements
- Noise Algorithm: [psrdnoise](https://github.com/stegu/psrdnoise)
- Continental Noise: ["Minecraft terrain generation in a nutshell"](https://youtu.be/CSa5O6knuwI?t=784)
- Erosion Trick: ["Better Mountain Generators That Aren't Perlin Noise or Erosion"](https://youtu.be/gsJHzBTPG0Y)
- Game Engine: [Bevy](https://bevy.org/)
- Physics Engine: [Avian](https://github.com/avianphysics/avian)
- Character Controller: [tnua](https://github.com/idanarye/bevy-tnua)