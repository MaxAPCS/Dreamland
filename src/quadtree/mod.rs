mod quad;
mod util;

pub use quad::*;

use bevy::math::IRect;
pub trait FromBounds {
    fn new(bounds: &IRect) -> Self;
}
