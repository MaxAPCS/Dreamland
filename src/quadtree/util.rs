use bevy::math::{IRect, IVec2};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct MIRect {
    pub rect: IRect,
}

impl MIRect {
    #[inline]
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self {
            rect: IRect::from_corners(IVec2::new(x0, y0), IVec2::new(x1, y1)),
        }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.rect.max.x - self.rect.min.x
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.rect.max.y - self.rect.min.y
    }

    #[inline]
    pub fn center(&self) -> IVec2 {
        (self.rect.min + self.rect.max) / 2
    }

    #[inline]
    pub fn contains(&self, point: &IVec2) -> bool {
        self.rect.min.x <= point.x
            && self.rect.max.x > point.x
            && self.rect.min.y <= point.y
            && self.rect.max.y > point.y
    }

    #[inline]
    pub fn contains_rect(&self, rect: &MIRect) -> bool {
        self.rect.min.x <= rect.rect.min.x
            && self.rect.max.x >= rect.rect.max.x
            && self.rect.min.y <= rect.rect.min.y
            && self.rect.max.y >= rect.rect.max.y
    }

    pub(super) fn sub_rect(&self, quadrant: Quadrant) -> MIRect {
        debug_assert!(self.width() >= 2 && self.height() >= 2);
        let IVec2 { x: mid_x, y: mid_y } = self.center();
        match quadrant {
            Quadrant::TopLft => MIRect::new(self.rect.min.x, mid_y, mid_x, self.rect.max.y),
            Quadrant::TopRht => MIRect::new(mid_x, mid_y, self.rect.max.x, self.rect.max.y),
            Quadrant::BotLft => MIRect::new(self.rect.min.x, self.rect.min.y, mid_x, mid_y),
            Quadrant::BotRht => MIRect::new(mid_x, self.rect.min.y, self.rect.max.x, mid_y),
        }
    }
}

impl From<IRect> for MIRect {
    fn from(rect: IRect) -> Self {
        Self { rect }
    }
}

impl Into<IRect> for MIRect {
    fn into(self) -> IRect {
        self.rect
    }
}

impl std::fmt::Debug for MIRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.rect.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Quadrant {
    TopLft,
    TopRht,
    BotLft,
    BotRht,
}

impl Quadrant {
    pub(super) fn from_point(rect: &MIRect, point: &IVec2) -> Option<Self> {
        if !rect.contains(point) {
            return None;
        }

        let IVec2 { x: mid_x, y: mid_y } = rect.center();
        match (point.x < mid_x, point.y < mid_y) {
            (true, false) => Some(Quadrant::TopLft),
            (false, false) => Some(Quadrant::TopRht),
            (true, true) => Some(Quadrant::BotLft),
            (false, true) => Some(Quadrant::BotRht),
        }
    }

    // MUST MATCH ORDER OF ENUM
    pub fn entries() -> [Quadrant; 4] {
        [
            Quadrant::TopLft,
            Quadrant::TopRht,
            Quadrant::BotLft,
            Quadrant::BotRht,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn quadrant_from_point() {
        let rect = MIRect::new(0, 0, 100, 100);
        let center = rect.center();

        let test_cases = [
            // Test each quadrant center
            (IVec2::new(25, 75), Quadrant::TopLft), // Top-left
            (IVec2::new(75, 75), Quadrant::TopRht), // Top-right
            (IVec2::new(25, 25), Quadrant::BotLft), // Bottom-left
            (IVec2::new(75, 25), Quadrant::BotRht), // Bottom-right
            // Points exactly on center lines should still map to quadrants
            (IVec2::new(center.x - 1, center.y - 1), Quadrant::BotLft),
            (IVec2::new(center.x, center.y - 1), Quadrant::BotRht),
            (IVec2::new(center.x - 1, center.y), Quadrant::TopLft),
            (IVec2::new(center.x, center.y), Quadrant::TopRht),
        ];

        for (point, expected_quadrant) in test_cases {
            assert_eq!(
                Quadrant::from_point(&rect, &point),
                Some(expected_quadrant),
                "Point {:?} should be in {:?}",
                point,
                expected_quadrant
            );
        }
    }

    #[wasm_bindgen_test]
    fn quadrant_from_point_outside() {
        let rect = MIRect::new(0, 0, 100, 100);
        let outside_points = [
            IVec2::new(-1, 50),  // Left
            IVec2::new(100, 50), // Right
            IVec2::new(50, -1),  // Top
            IVec2::new(50, 100), // Bottom
        ];

        for point in outside_points {
            assert!(
                Quadrant::from_point(&rect, &point).is_none(),
                "Point {:?} should be outside bounds",
                point
            );
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn quadrant_mirect_consistency() {
        let rect = MIRect::new(0, 0, 100, 100);
        let points = [
            IVec2::new(0, 0),
            IVec2::new(50, 50),
            IVec2::new(99, 99),
            IVec2::new(0, 99),
            IVec2::new(99, 0),
            IVec2::new(25, 25),
            IVec2::new(75, 75),
        ];

        for point in points {
            assert!(
                rect.sub_rect(Quadrant::from_point(&rect, &point).unwrap())
                    .contains(&point),
                "Point {:?} should be contained in sub-rectangle",
                point
            );
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn quadrant_entries_consistency() {
        for quad in Quadrant::entries() {
            assert_eq!(
                Quadrant::entries()[quad as usize],
                quad,
                "Quadrant {:?} should be at its own index in entries.",
                quad
            );
        }
    }

    #[wasm_bindgen_test]
    fn mirect_sub_rect() {
        let parent = MIRect::new(0, 0, 100, 100);

        let test_cases = [
            (Quadrant::TopLft, MIRect::new(0, 50, 50, 100)),
            (Quadrant::TopRht, MIRect::new(50, 50, 100, 100)),
            (Quadrant::BotLft, MIRect::new(0, 0, 50, 50)),
            (Quadrant::BotRht, MIRect::new(50, 0, 100, 50)),
        ];

        for (quadrant, expected) in test_cases {
            assert_eq!(
                parent.sub_rect(quadrant),
                expected,
                "Quadrant {:?} should produce correct sub-rect",
                quadrant
            );
        }
    }
}
