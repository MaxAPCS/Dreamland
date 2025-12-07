struct FractalNoiseConfig {
    seed: f32,       // (,)
    octaves: u32,    // [1,)
    lacunarity: f32, // (1,)
    gain: f32,       // (0,1)
    erosion: f32,    // [0,) lower = more fine detail
    scale: f32,      // (0,) lower = shorter features
    macroscale: f32, // (,)
}

const MOUNTAINS: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 6,
    lacunarity: 1.8,
    gain: 0.5,
    erosion: 0.01 / 24., // pre-divide scale.
    scale: 24.,
    macroscale: 128.,
};

const FOOTHILLS: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 4,
    lacunarity: 2.,
    gain: 0.4,
    erosion: 0.,
    scale: 12.,
    offset: 0.0,
};

const TUNDRA: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 4,
    lacunarity: 1.2,
    gain: 0.3,
    erosion: 0.3 / 6., // pre-divide scale.
    scale: 6.,
    macroscale: 128.,
};
