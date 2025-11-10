use bevy::math::{IRect, IVec2};

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
        debug_assert!(parent.width() >= 2 && parent.height() >= 2);
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

pub struct Quad<T> {
    bounds: IRect,
    node: Node<T>,
}

impl<T> Quad<T> {
    // find the quad with these bounds
    pub fn matching_bounds(&mut self, bounds: &IRect) -> LocationResult<&mut Quad<T>> {
        if self.bounds == *bounds {
            return LocationResult::Found(self);
        }

        if !self.contains(bounds) {
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

    pub fn get_bounds(&self) -> &IRect {
        &self.bounds
    }

    pub fn contains(&self, rect: &IRect) -> bool {
        self.bounds.min.x <= rect.min.x
            && self.bounds.max.x >= rect.max.x
            && self.bounds.min.y <= rect.min.y
            && self.bounds.max.y >= rect.max.y
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
    pub fn new(bounds: IRect) -> Self {
        Self {
            bounds,
            node: Node::Leaf(T::new(&bounds)),
        }
    }

    fn subdivide(&mut self) {
        debug_assert!(self.is_leaf());
        self.node = Node::Branch(
            Quadrant::entries().map(|q| Box::new(Quad::new(q.sub_rect(&self.bounds)))),
        );
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

impl<T: std::fmt::Display> std::fmt::Debug for Quad<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:>3}, {:>3}) {}x{} {}",
            self.bounds.min.x as f32 / 128.,
            self.bounds.min.y as f32 / 128.,
            self.bounds.width() as f32 / 128.,
            self.bounds.height() as f32 / 128.,
            match &self.node {
                Node::Leaf(val) => format!("{}", val),
                Node::Branch(_) => "B".to_string(),
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
    fn test_quadrant_contains_consistency() {
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
            (IRect::new(0, 0, 100, 100), IVec2::splat(50)),
            (IRect::new(0, 0, 10, 10), IVec2::new(0, 9)),
            (IRect::new(-128, -128, 128, 128), IVec2::ZERO),
            (
                IRect::new(-128 - 256, -128 - 256, -128, -128),
                IVec2::new(-250, -150),
            ),
            (IRect::new(0, 0, 2, 2), IVec2::new(0, 0)),
            (IRect::new(0, 0, 2, 2), IVec2::new(0, 1)),
            (IRect::new(0, 0, 2, 2), IVec2::new(1, 0)),
            (IRect::new(0, 0, 2, 2), IVec2::new(1, 1)),
            (IRect::new(100, 100, 200, 200), IVec2::new(100, 100)),
            (IRect::new(100, 100, 200, 200), IVec2::new(199, 199)),
            (IRect::new(-100, -100, 0, 0), IVec2::new(-100, -100)),
            (IRect::new(-100, -100, 0, 0), IVec2::new(-1, -1)),
        ];
        for (bounds, point) in testbounds {
            let mut quad = Quad::<Dummy>::new(bounds);
            quad.subdivide();

            let Node::Branch(children) = &quad.node else {
                panic!("Quad should be branch");
            };

            let quadrant = Quadrant::from_point(&quad.bounds, &point).unwrap();
            assert!(
                quadrant.sub_rect(&bounds).contains(point),
                "Quadrant doesn't contain point it was formed from"
            );
            assert!(
                children[quadrant as usize].bounds.contains(point),
                "Quad's child doesn't contain point within quadrant"
            );
        }
    }

    #[derive(PartialEq, Eq, Debug, Clone)]
    struct Dummy(i32);

    impl FromBounds for Dummy {
        fn new(_: &IRect) -> Self {
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
        let rect = IRect::new(0, 0, 100, 100);

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
        let rect = IRect::new(0, 0, 100, 100);
        let outside_points = [
            IVec2::new(-1, 50),  // Left
            IVec2::new(101, 50), // Right
            IVec2::new(50, -1),  // Top
            IVec2::new(50, 101), // Bottom
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
        let rect = IRect::new(0, 0, 100, 100);
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
        let parent = IRect::new(0, 0, 100, 100);

        let test_cases = [
            (Quadrant::TopLft, IRect::new(0, 50, 50, 100)),
            (Quadrant::TopRht, IRect::new(50, 50, 100, 100)),
            (Quadrant::BotLft, IRect::new(0, 0, 50, 50)),
            (Quadrant::BotRht, IRect::new(50, 0, 100, 50)),
        ];

        for (quadrant, expected) in test_cases {
            assert_eq!(
                quadrant.sub_rect(&parent),
                expected,
                "Quadrant {:?} should produce correct sub-rect",
                quadrant
            );
        }
    }

    #[wasm_bindgen_test]
    fn test_quadrant_entries_order() {
        let entries = Quadrant::entries();
        assert_eq!(entries[0], Quadrant::TopLft);
        assert_eq!(entries[1], Quadrant::TopRht);
        assert_eq!(entries[2], Quadrant::BotLft);
        assert_eq!(entries[3], Quadrant::BotRht);
    }

    // ===== Quad Tree Creation & Basic Properties =====

    #[wasm_bindgen_test]
    fn test_quadtree_creation() {
        let bounds = IRect::new(0, 0, 100, 100);
        let quad = Quad::<Dummy>::new(bounds);

        assert!(quad.is_leaf());
        assert_eq!(quad.get_bounds(), &bounds);
        assert_eq!(quad.get_leaf(), Some(&Dummy(0)));
    }

    #[wasm_bindgen_test]
    fn test_quadtree_leaf_operations() {
        let bounds = IRect::new(0, 0, 100, 100);
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
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let point = IVec2::new(50, 50);

        // Setting min depth 0 should return self (root)
        let result = quad.set_min_depth(&point, 0);
        assert!(std::ptr::eq(result, &quad));
        assert!(quad.is_leaf()); // Should remain leaf at depth 0
    }

    #[wasm_bindgen_test]
    fn test_set_min_depth_multiple_levels() {
        let bounds = IRect::new(0, 0, 100, 100);
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
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Test points exactly on quadrant boundaries
        let edge_points = [
            IVec2::new(50, 50),  // Exact center
            IVec2::new(0, 50),   // Left edge
            IVec2::new(50, 0),   // Bottom edge
            IVec2::new(50, 100), // Top edge
            IVec2::new(100, 50), // Right edge
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
        let bounds = IRect::new(0, 0, 100, 100);
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
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        let result = quad.matching_bounds(&bounds);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_partial_overlap() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let key = 42;
        quad.set_leaf(Dummy(key));

        // Bounds that cross multiple quadrants
        let crossing_bounds = IRect::new(25, 25, 75, 75);
        let result = quad.matching_bounds(&crossing_bounds);
        let LocationResult::Closest(result) = result else {
            panic!("Expected Closest, got {:?}", result);
        };
        assert_eq!(result.get_leaf().expect("Expected Leaf").0, key);
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_same_quadrant() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Subdivide first
        quad.subdivide();

        // Test bounds that fit entirely in one quadrant
        let top_left_bounds = IRect::new(0, 50, 50, 100);
        let result = quad.matching_bounds(&top_left_bounds);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &top_left_bounds);
        }
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_non_existent() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Bounds completely outside
        let outside_bounds = IRect::new(200, 200, 300, 300);
        let result = quad.matching_bounds(&outside_bounds);
        assert!(matches!(result, LocationResult::NotFound));
    }

    // ===== Traversal Tests =====

    #[wasm_bindgen_test]
    fn test_for_each_on_leaf() {
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);
        let mut count = 0;

        quad.for_each_mut(&mut |_| count += 1);

        assert_eq!(count, 1);
    }

    // ===== Edge Cases =====

    #[wasm_bindgen_test]
    #[should_panic(expected = "assertion failed")]
    fn test_single_pixel_bounds() {
        let bounds = IRect::new(0, 0, 1, 1);
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
        let bounds = IRect::new(0, 0, 100, 100);
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
        let bounds = IRect::new(0, 0, 128, 128); // Power of 2 for clean division
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
        let bounds = IRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::new(bounds);

        // Convert to branch
        quad.subdivide();

        // This should panic because we're a branch, not a leaf
        quad.set_leaf(Dummy(42));
    }
}
