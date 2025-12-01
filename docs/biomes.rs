struct FractalNoiseConfig {
    seed: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
    erosion: f32, // lower = more fine detail
    scale: f32,   // lower = less height
    offset: f32,
}

const MOUNTAINS: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 6,
    lacunarity: 1.7,
    gain: 0.7,
    erosion: 0.4,
    scale: 24.,
    offset: 0.0,
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
