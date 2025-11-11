use bevy::math::{IRect, IVec2};

pub struct Quad<T> {
    bounds: MIRect,
    node: Node<T>,
}

impl<T> Quad<T> {
    // find the quad with these bounds
    pub fn matching_bounds(&mut self, bounds: &IRect) -> LocationResult<&mut Quad<T>> {
        if self.bounds.rect == *bounds {
            return LocationResult::Found(self);
        }

        if !self.bounds.contains_rect(&(*bounds).into()) {
            return LocationResult::NotFound;
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
        debug_assert!(self.bounds.contains(point));

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

    fn is_leaf(&self) -> bool {
        matches!(self.node, Node::Leaf(_))
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

    pub fn get_bounds(&self) -> &MIRect {
        &self.bounds
    }

    pub fn for_each<F: FnMut(&Quad<T>)>(&self, callback: &mut F) {
        match &self.node {
            Node::Branch(children) => {
                for child in children {
                    child.for_each(callback);
                }
            }
            Node::Leaf(_) => callback(self),
        }
    }

    pub fn for_each_mut<F: FnMut(&mut Quad<T>)>(&mut self, callback: &mut F) {
        match &mut self.node {
            Node::Branch(children) => {
                for child in children {
                    child.for_each_mut(callback);
                }
            }
            Node::Leaf(_) => callback(self),
        }
    }
}

impl<T: FromBounds> Quad<T> {
    pub fn new(bounds: MIRect) -> Self {
        Self {
            node: Node::Leaf(T::new(&bounds)),
            bounds,
        }
    }

    fn subdivide(&mut self) {
        debug_assert!(self.is_leaf());
        self.node =
            Node::Branch(Quadrant::entries().map(|q| Box::new(Quad::new(self.bounds.sub_rect(q)))));
    }

    fn unite(&mut self) {
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
        debug_assert!(self.bounds.contains(point));

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

pub trait FromBounds {
    fn new(bounds: &MIRect) -> Self;
}

impl<T: std::fmt::Display> std::fmt::Debug for Quad<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:>3}, {:>3}) {}x{} {}",
            self.bounds.rect.min.x as f32 / 128.,
            self.bounds.rect.min.y as f32 / 128.,
            self.bounds.width() as f32 / 128.,
            self.bounds.height() as f32 / 128.,
            match &self.node {
                Node::Leaf(val) => format!("{}", val),
                Node::Branch(_) => "B".to_string(),
            }
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MIRect {
    rect: IRect,
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

    #[inline]
    pub fn union(&self, other: Self) -> Self {
        Self {
            rect: IRect {
                min: self.rect.min.min(other.rect.min),
                max: self.rect.max.max(other.rect.max),
            },
        }
    }

    fn sub_rect(&self, quadrant: Quadrant) -> MIRect {
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

#[derive(PartialEq, Debug)]
pub enum LocationResult<T> {
    Found(T),
    Closest(T),
    NotFound,
}

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
    fn from_point(rect: &MIRect, point: &IVec2) -> Option<Self> {
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
    fn entries() -> [Quadrant; 4] {
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

    #[wasm_bindgen_test(unsupported = test)]
    fn test_quadrant_contains_consistency() {
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
            let q = Quadrant::from_point(&rect, &point);
            assert!(q.is_some(), "Point {:?} should map to a quadrant", point);
            let q = q.unwrap();
            let sub_rect = rect.sub_rect(q);
            assert!(
                sub_rect.contains(&point),
                "Point {:?} should be contained in sub-rectangle {:?}",
                point,
                sub_rect
            );
        }

        for point in points {
            assert!(
                rect.sub_rect(Quadrant::from_point(&rect, &point).unwrap())
                    .contains(&point),
                "Subrect of {:?} should contain it.",
                point
            );
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_quadrant_order_consistency() {
        for quad in Quadrant::entries() {
            assert_eq!(
                Quadrant::entries()[quad as usize],
                quad,
                "Quadrant {:?} should be at its own index in entries.",
                quad
            );
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn test_quad_child_contains() {
        let testbounds = [
            (MIRect::new(0, 0, 100, 100), IVec2::splat(50)),
            (MIRect::new(0, 0, 10, 10), IVec2::new(0, 9)),
            (MIRect::new(-128, -128, 128, 128), IVec2::ZERO),
            (
                MIRect::new(-128 - 256, -128 - 256, -128, -128),
                IVec2::new(-250, -150),
            ),
            (MIRect::new(0, 0, 2, 2), IVec2::new(0, 0)),
            (MIRect::new(0, 0, 2, 2), IVec2::new(0, 1)),
            (MIRect::new(0, 0, 2, 2), IVec2::new(1, 0)),
            (MIRect::new(0, 0, 2, 2), IVec2::new(1, 1)),
            (MIRect::new(100, 100, 200, 200), IVec2::new(100, 100)),
            (MIRect::new(100, 100, 200, 200), IVec2::new(199, 199)),
            (MIRect::new(-100, -100, 0, 0), IVec2::new(-100, -100)),
            (MIRect::new(-100, -100, 0, 0), IVec2::new(-1, -1)),
        ];
        for (bounds, point) in testbounds {
            let mut quad = Quad::<Dummy>::new(bounds);
            quad.subdivide();

            let Node::Branch(children) = &quad.node else {
                panic!("Quad should be branch");
            };

            let quadrant = Quadrant::from_point(&quad.bounds, &point).unwrap();
            assert!(
                bounds.sub_rect(quadrant).contains(&point),
                "Quadrant doesn't contain point it was formed from"
            );
            for (i, child) in children.iter().enumerate() {
                if i == quadrant as usize {
                    assert!(
                        child.bounds.contains(&point),
                        "Quad's child doesn't contain point within quadrant"
                    );
                } else {
                    assert!(
                        !child.bounds.contains(&point),
                        "Quad's child contains point within other quadrant"
                    );
                }
            }
        }
    }

    #[derive(PartialEq, Eq, Debug, Clone)]
    struct Dummy(i32);

    impl FromBounds for Dummy {
        fn new(_: &MIRect) -> Self {
            Self(0)
        }
    }

    impl std::fmt::Display for Dummy {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Dummy").field("value", &self.0).finish()
        }
    }

    // ===== Quadrant Tests =====

    #[wasm_bindgen_test]
    fn test_quadrant_from_point_within_bounds() {
        let rect = MIRect::new(0, 0, 100, 100);

        // Test each quadrant center
        let test_cases = [
            (IVec2::new(25, 75), Quadrant::TopLft), // Top-left
            (IVec2::new(75, 75), Quadrant::TopRht), // Top-right
            (IVec2::new(25, 25), Quadrant::BotLft), // Bottom-left
            (IVec2::new(75, 25), Quadrant::BotRht), // Bottom-right
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
    fn test_quadrant_from_point_outside_bounds() {
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

    #[wasm_bindgen_test]
    fn test_quadrant_from_point_on_boundaries() {
        let rect = MIRect::new(0, 0, 100, 100);
        let center = rect.center();

        // Points exactly on center lines should still map to quadrants
        let boundary_cases = [
            (IVec2::new(center.x - 1, center.y - 1), Quadrant::BotLft),
            (IVec2::new(center.x, center.y - 1), Quadrant::BotRht),
            (IVec2::new(center.x - 1, center.y), Quadrant::TopLft),
            (IVec2::new(center.x, center.y), Quadrant::TopRht),
        ];

        for (point, expected) in boundary_cases {
            assert_eq!(
                Quadrant::from_point(&rect, &point),
                Some(expected),
                "Boundary point {:?} should map correctly",
                point
            );
        }
    }

    #[wasm_bindgen_test]
    fn test_quadrant_sub_rect_correctness() {
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

    // ===== Quad Tree Creation & Basic Properties =====

    #[wasm_bindgen_test]
    fn test_quadtree_creation() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let quad = Quad::<Dummy>::new(bounds);

        assert!(quad.is_leaf());
        assert_eq!(quad.get_bounds(), &bounds);
        assert_eq!(quad.get_leaf(), Some(&Dummy::new(&bounds)));
    }

    #[wasm_bindgen_test]
    fn test_quadtree_leaf_operations() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Test initial state
        assert_eq!(quad.get_leaf(), Some(&Dummy(0)));

        // Test setting leaf value
        quad.set_leaf(Dummy(42));
        assert_eq!(quad.get_leaf(), Some(&Dummy(42)));

        // Test that branch nodes return None for get_leaf
        quad.subdivide();
        assert_eq!(quad.get_leaf(), None);
    }

    // ===== Quad Tree Depth Operations =====

    #[wasm_bindgen_test]
    fn test_set_min_depth_zero() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let point = IVec2::new(50, 50);

        // Setting min depth 0 should return self (root)
        let result = quad.set_min_depth(&point, 0);
        assert!(std::ptr::eq(result, &quad));
        assert!(quad.is_leaf()); // Should remain leaf at depth 0
    }

    #[wasm_bindgen_test]
    fn test_set_min_depth_multiple_levels() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let point = IVec2::new(25, 75); // Top-left quadrant

        // Set depth to 2 (creates 2 levels of subdivision)
        let _parent = quad.set_min_depth(&point, 2);

        // Root should be branched
        assert!(!quad.is_leaf());

        // Verify the hierarchy was created correctly
        if let Node::Branch(children) = &quad.node {
            // First level - top-left quadrant should be branched
            let top_left = &children[0];
            assert!(!top_left.is_leaf());

            if let Node::Branch(grand_children) = &top_left.node {
                // Second level - should find our target point's quadrant
                let target_quadrant = Quadrant::from_point(top_left.get_bounds(), &point).unwrap();
                let final_child = &grand_children[target_quadrant as usize];
                assert!(final_child.is_leaf());
            } else {
                panic!("Expected second level branching");
            }
        } else {
            panic!("Expected first level branching");
        }
    }

    #[wasm_bindgen_test]
    fn test_set_min_depth_point_on_edge() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Test points exactly on quadrant boundaries
        let edge_points = [
            IVec2::new(50, 50), // Exact center
            IVec2::new(0, 50),  // Left edge
            IVec2::new(50, 0),  // Bottom edge
        ];

        for point in edge_points {
            let _result = quad.set_min_depth(&point, 1);
            assert!(
                !quad.is_leaf(),
                "Should subdivide for edge point {:?}",
                point
            );

            // Reset for next test
            quad = Quad::<Dummy>::new(bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_set_max_depth_merging() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let point = IVec2::new(25, 75);

        // First create some depth
        quad.set_min_depth(&point, 2);
        assert!(!quad.is_leaf());

        // Then set max depth to 1 (should merge back one level)
        quad.set_max_depth(&point, 1);

        // Root should still be branched (depth 1)
        assert!(!quad.is_leaf());

        // But the direct child containing our point should now be a leaf
        if let Node::Branch(children) = &quad.node {
            let target_quadrant = Quadrant::from_point(&bounds, &point).unwrap();
            assert!(children[target_quadrant as usize].is_leaf());
        }
    }

    // ===== Bounds Matching Tests =====

    #[wasm_bindgen_test]
    fn test_matching_bounds_exact_match() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        let result = quad.matching_bounds(&bounds.rect);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_partial_overlap() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let key = 42;
        quad.set_leaf(Dummy(key));

        // Bounds that cross multiple quadrants
        let crossing_bounds = MIRect::new(25, 25, 75, 75);
        let result = quad.matching_bounds(&crossing_bounds.rect);
        let LocationResult::Closest(result) = result else {
            panic!("Expected Closest, got {:?}", result);
        };
        assert_eq!(result.get_leaf().expect("Expected Leaf").0, key);
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_same_quadrant() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Subdivide first
        quad.subdivide();

        // Test bounds that fit entirely in one quadrant
        let top_left_bounds = MIRect::new(0, 50, 50, 100);
        let result = quad.matching_bounds(&top_left_bounds.rect);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &top_left_bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_non_existent() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Bounds completely outside
        let outside_bounds = MIRect::new(200, 200, 300, 300);
        let result = quad.matching_bounds(&outside_bounds.rect);
        assert!(matches!(result, LocationResult::NotFound));
    }

    // ===== Traversal Tests =====

    #[wasm_bindgen_test]
    fn test_for_each_on_leaf() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let quad = Quad::<Dummy>::new(bounds);
        let mut count = 0;

        quad.for_each(&mut |_| count += 1);

        assert_eq!(count, 1);
    }

    // ===== Edge Cases =====

    #[wasm_bindgen_test]
    #[should_panic(expected = "assertion failed")]
    fn test_single_pixel_bounds() {
        let bounds = MIRect::new(0, 0, 1, 1);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Should be able to handle single pixel
        assert!(quad.is_leaf());

        let point = IVec2::new(0, 0);
        let result = quad.set_min_depth(&point, 1); // should panic
        assert!(std::ptr::eq(result, &quad));
        // Single pixel cannot be subdivided further
        assert!(quad.is_leaf());
    }

    #[wasm_bindgen_test]
    fn test_point_at_corners() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        let corners = [
            IVec2::new(0, 0),   // Bottom-left
            IVec2::new(99, 0),  // Bottom-right
            IVec2::new(0, 99),  // Top-left
            IVec2::new(99, 99), // Top-right
        ];

        for corner in corners {
            let _result = quad.set_min_depth(&corner, 1);
            assert!(!quad.is_leaf(), "Should subdivide for corner {:?}", corner);

            // Reset for next test
            quad = Quad::<Dummy>::new(bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_deep_nesting() {
        let bounds = MIRect::new(0, 0, 128, 128); // Power of 2 for clean division
        let mut quad = Quad::<Dummy>::new(bounds);
        let point = IVec2::new(1, 127); // Near top-left corner

        // Test very deep nesting (5 levels)
        quad.set_min_depth(&point, 5);

        // Verify we can traverse without panic
        let mut leaf_count = 0;
        quad.for_each_mut(&mut |node| {
            assert!(node.is_leaf());
            leaf_count += 1;
        });

        // Should have 2^(depth - 1) leaves
        assert_eq!(leaf_count, 2_i32.pow(5 - 1));
    }

    #[wasm_bindgen_test]
    #[should_panic(expected = "assertion failed")]
    fn test_set_leaf_on_branch_panics() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Convert to branch
        quad.subdivide();

        // This should panic because we're a branch, not a leaf
        quad.set_leaf(Dummy(42));
    }

    #[wasm_bindgen_test]
    fn test_quad_parent_child_contains() {
        let bounds = MIRect::new(0, 0, 101, 99);
        let mut quad = Quad::<Dummy>::new(bounds);
        quad.subdivide();

        for x in bounds.rect.min.x - 1..bounds.rect.max.x + 1 {
            for y in bounds.rect.min.y - 1..bounds.rect.max.y + 1 {
                let pt = &IVec2::new(x, y);

                let Node::Branch(children) = &quad.node else {
                    panic!();
                };
                let mut i = 0;
                for child in children {
                    i += child.bounds.contains(pt) as i32;
                }

                assert_eq!(i, quad.bounds.contains(pt) as i32);
            }
        }
    }
}
