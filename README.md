# Dreamland
**A 3D procedural world generator using Rust, Bevy, WASM, and WebGPU.**

Dreamland is an infinite world of procedural terrain that continuously generates around the user, rendering in high quality close by and tapering off when far away. Fractal Brownian Motion is applied to simplex noise to sample heights in parallel on the GPU, which are passed back to the CPU and turned into triangle strip meshes with smooth shading. These are dynamically generated and destroyed at variable detail levels by an efficient lazy quadtree algorithm, permitting nearly unlimited world size.
This project is designed firstly to be a personal foray into the subject, and perhaps alternatively as a flexible visualization of terrain generators for game developers, 3D artists, and others. This experience is designed to be accessible on the web, enabling greater accessibility.
Dreamland works on Firefox (with flags), does not work on Chromium-based browsers, and is untested on Safari.

## Roadmap

As of now, the project has all of its core features, and I would consider it complete. Work will continue to add polish wherever possible.

### Complete
- CPU Noise Generation
- Mesh Construction
- Physics Engine Integration
- Character Controller
- Procedural Atmosphere
- GPU Compute Noise Generation
- Quadtree LOD Framework
- Dynamic LOD Centering
- Infinite Generation
- [Analytical Derivative Erosion Trick](https://www.youtube.com/watch?v=gsJHzBTPG0Y)

### In Progress

### Planned
- Procedural Foliage
- Profiling
- Procedural Flat Clouds
- Mobile Device Accelerometer Camera Control
- Parameterized Noise Settings
- Terrain Coloring
- Curved World Vertex Shader

## Acknowledgements
Noise Algorithm: [psrdnoise](https://github.com/stegu/psrdnoise)
