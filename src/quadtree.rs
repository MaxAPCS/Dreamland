use bevy::math::{IRect, IVec2};

pub enum LocationResult<T> {
    Found(T),
    Closest(T),
    NotFound,
}

#[derive(Debug)]
enum Node<T> {
    Branch([Box<Quad<T>>; 4]),
    Leaf(T),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Quadrant {
    TopLft,
    TopRht,
    BotLft,
    BotRht,
}

impl Quadrant {
    fn from_point(rect: &IRect, point: &IVec2) -> Option<Self> {
        if !rect.contains(*point) {
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

    fn sub_rect(self, parent: &IRect) -> IRect {
        let IVec2 { x: mid_x, y: mid_y } = parent.center();
        match self {
            Quadrant::TopLft => IRect::new(parent.min.x, mid_y, mid_x, parent.max.y),
            Quadrant::TopRht => IRect::new(mid_x, mid_y, parent.max.x, parent.max.y),
            Quadrant::BotLft => IRect::new(parent.min.x, parent.min.y, mid_x, mid_y),
            Quadrant::BotRht => IRect::new(mid_x, parent.min.y, parent.max.x, mid_y),
        }
    }

    // MUST MATCH ORDER OF ENUM
    fn entries() -> [Quadrant; 4] {
        [
            Quadrant::TopLft,
            Quadrant::TopRht,
            Quadrant::BotLft,
            Quadrant::BotRht,
        ]
    }
}

#[derive(Debug)]
pub struct Quad<T> {
    bounds: IRect,
    node: Node<T>,
}

impl<T> Quad<T> {
    fn is_leaf(&self) -> bool {
        matches!(self.node, Node::Leaf(_))
    }

    // find the quad with these bounds
    pub fn matching_bounds(&mut self, bounds: &IRect) -> LocationResult<&mut Quad<T>> {
        if self.bounds == *bounds {
            return LocationResult::Found(self);
        }

        if self.is_leaf() {
            return LocationResult::Closest(self);
        }

        let Node::Branch(children) = &mut self.node else {
            unreachable!()
        };

        match (
            Quadrant::from_point(&self.bounds, &bounds.min),
            Quadrant::from_point(&self.bounds, &(bounds.max - IVec2::ONE)),
        ) {
            (Some(min), Some(max)) if min == max => children[min as usize].matching_bounds(bounds),
            _ => LocationResult::NotFound,
        }
    }

    // find the smallest quad depth >= target containing point
    fn smallest_containing_at_depth(
        &mut self,
        point: &IVec2,
        target_depth: u32,
        current_depth: u32,
    ) -> &mut Quad<T> {
        debug_assert!(self.bounds.contains(*point));

        // if deepest we can go OR already deep enough
        if self.is_leaf() || current_depth >= target_depth {
            return self;
        }

        let Node::Branch(children) = &mut self.node else {
            unreachable!()
        };

        // find quadrant point belongs to
        let quadrant = Quadrant::from_point(&self.bounds, &point).unwrap() as usize;

        // go deeper
        children[quadrant].smallest_containing_at_depth(point, target_depth, current_depth + 1)
    }

    pub fn set_leaf(&mut self, value: T) {
        debug_assert!(self.is_leaf());
        self.node = Node::Leaf(value);
    }

    pub fn get_leaf(&self) -> Option<&T> {
        match &self.node {
            Node::Leaf(value) => Some(value),
            Node::Branch(_) => None,
        }
    }

    pub fn get_bounds(&self) -> &IRect {
        &self.bounds
    }

    pub fn for_each<F: FnMut(&mut Quad<T>)>(&mut self, callback: &mut F) {
        match &mut self.node {
            Node::Branch(children) => {
                for child in children {
                    child.for_each(callback);
                }
            }
            Node::Leaf(_) => callback(self),
        }
    }
}

impl<T: FromBounds> Quad<T> {
    pub fn new(bounds: IRect) -> Self {
        Self {
            bounds,
            node: Node::Leaf(T::new(&bounds)),
        }
    }

    fn subdivide(&mut self) {
        bevy::log::debug!("SUBDV {}", self);
        debug_assert!(self.is_leaf());
        self.node = Node::Branch(
            Quadrant::entries().map(|q| Box::new(Quad::new(q.sub_rect(&self.bounds)))),
        );
    }

    fn unite(&mut self) {
        bevy::log::debug!("UNITE {}", self);
        debug_assert!(!self.is_leaf());
        self.node = Node::Leaf(T::new(&self.bounds));
    }

    /// NOTE: ONLY CALL ON PARENT NODE - assumes starting depth = 0
    pub fn set_min_depth(&mut self, point: &IVec2, target_depth: u32) -> &Quad<T> {
        self.set_min_depth_recurse(point, target_depth, 0)
    }

    /// Recursively ensures the quadtree node containing `point` has a depth of at least `target_depth`.
    ///
    /// # Arguments
    /// * `point` - The point to locate within the quadtree.
    /// * `target_depth` - The minimum depth (inclusive) the containing node must achieve.
    /// * `current_depth` - Current recursion depth (starts at 0 for root node).
    ///
    /// # Returns
    /// A reference to the parent of the leaf node at the target depth containing `point`.
    ///
    /// # Example
    /// ```
    /// // Ensures point (x,y) has at least depth 3
    /// quad.set_min_depth(&IVec2::new(x, y), 3);
    /// ```
    fn set_min_depth_recurse(
        &mut self,
        point: &IVec2,
        target_depth: u32,
        current_depth: u32,
    ) -> &Quad<T> {
        debug_assert!(self.bounds.contains(*point));

        if current_depth >= target_depth {
            return self;
        }

        if self.is_leaf() {
            self.subdivide();
        }

        if current_depth == target_depth - 1 {
            return self;
        }

        let Node::Branch(children) = &mut self.node else {
            unreachable!();
        };

        let quadrant = Quadrant::from_point(&self.bounds, point).unwrap() as usize;
        return children[quadrant].set_min_depth_recurse(point, target_depth, current_depth + 1);
    }

    pub fn set_max_depth(&mut self, point: &IVec2, target_depth: u32) {
        let smallest = self.smallest_containing_at_depth(point, target_depth, 0);

        if let Node::Branch(_) = &smallest.node {
            smallest.unite();
        }
    }
}

impl<T> std::fmt::Display for Quad<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:>3}, {:>3}) {}x{} {}",
            self.bounds.min.x as f32 / 128.,
            self.bounds.min.y as f32 / 128.,
            self.bounds.width() as f32 / 128.,
            self.bounds.height() as f32 / 128.,
            match self.is_leaf() {
                true => "L",
                false => "B",
            }
        )
    }
}

pub trait FromBounds {
    fn new(bounds: &IRect) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test(unsupported = test)]
    fn test_quadrant_consistency() {
        let rect = IRect::new(0, 0, 100, 100);
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
            let q = Quadrant::from_point(&rect, &point);
            assert!(q.is_some(), "Point {:?} should map to a quadrant", point);
            let q = q.unwrap();
            let sub_rect = q.sub_rect(&rect);
            assert!(
                sub_rect.contains(point),
                "Point {:?} should be contained in sub-rectangle {:?}",
                point,
                sub_rect
            );
        }

        for point in points {
            assert!(
                Quadrant::from_point(&rect, &point)
                    .unwrap()
                    .sub_rect(&rect)
                    .contains(point),
                "Subrect of {:?} should contain it.",
                point
            );
        }

        for quad in Quadrant::entries() {
            assert_eq!(
                Quadrant::entries()[quad as usize],
                quad,
                "Quadrant {:?} should be at its own index in entries.",
                quad
            );
        }
    }

    #[derive(PartialEq, Debug)]
    struct Dummy(i32);
    impl FromBounds for Dummy {
        fn new(_: &IRect) -> Self {
            Self(0)
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_quadtree_creation_and_depth() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        assert!(quad.is_leaf());
        assert_eq!(quad.get_bounds(), &bounds);

        // Test setting minimum depth
        let point = IVec2::new(50, 50);
        let _parent = quad.set_min_depth(&point, 2);

        assert!(!quad.is_leaf());
        assert_eq!(quad.get_bounds(), &bounds);
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_leaf_value_setting() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        assert_eq!(quad.get_leaf(), Some(&Dummy(0)));

        quad.set_leaf(Dummy(42));
        assert_eq!(quad.get_leaf(), Some(&Dummy(42)));
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_bounds_matching() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        let matching_bounds = IRect::new(0, 0, 100, 100);
        let result = quad.matching_bounds(&matching_bounds);
        assert!(matches!(result, LocationResult::Found(_)));
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_for_each() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let mut count = 0;

        quad.for_each(&mut |_| {
            count += 1;
        });

        assert_eq!(count, 1);

        quad.subdivide();

        count = 0;
        quad.for_each(&mut |_| {
            count += 1;
        });

        assert_eq!(count, 4);
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_subdivide_and_unite() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        quad.subdivide();
        assert!(!quad.is_leaf());

        quad.unite();
        assert!(quad.is_leaf());
    }
}
