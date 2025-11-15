use bevy::math::IVec2;

use crate::quadtree::{
    FromBounds,
    util::{MIRect, Quadrant},
};

#[derive(Debug)]
pub struct Quad<T> {
    bounds: MIRect,
    node: Node<T>,
}

#[derive(Debug)]
pub(super) enum Node<T> {
    Branch([Box<Quad<T>>; 4]),
    Leaf(T),
}

impl<T> Quad<T> {
    // find the quad with these bounds
    pub fn matching_bounds(&mut self, bounds: &bevy::math::IRect) -> LocationResult<&mut Quad<T>> {
        if self.bounds.rect == *bounds {
            return LocationResult::Found(self);
        }

        let bounds = &(*bounds).into();
        if !self.bounds.contains_rect(bounds) {
            return LocationResult::NotFound;
        }

        if self.is_leaf() {
            return LocationResult::Closest(self);
        }

        let Node::Branch(children) = &mut self.node else {
            unreachable!()
        };

        match (
            Quadrant::from_point(&self.bounds, &bounds.rect.min),
            Quadrant::from_point(&self.bounds, &(bounds.rect.max - IVec2::ONE)),
        ) {
            (Some(min), Some(max)) if min == max => {
                children[min as usize].matching_bounds(&bounds.rect)
            }
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

    pub fn is_leaf(&self) -> bool {
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

    pub fn get_bounds(&self) -> &bevy::math::IRect {
        &self.bounds.rect
    }

    pub fn contains(&self, point: &IVec2) -> bool {
        self.bounds.contains(point)
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
    pub fn new(size: IVec2) -> Self {
        Quad::from_bounds(bevy::math::IRect::from_center_size(IVec2::ZERO, size).into())
    }

    fn from_bounds(bounds: MIRect) -> Self {
        Self {
            node: Node::Leaf(T::new(&bounds.rect)),
            bounds: bounds,
        }
    }

    fn subdivide(&mut self) {
        debug_assert!(self.is_leaf());
        self.node = Node::Branch(
            Quadrant::entries().map(|q| Box::new(Quad::from_bounds(self.bounds.sub_rect(q)))),
        );
    }

    fn unite(&mut self) {
        debug_assert!(!self.is_leaf());
        self.node = Node::Leaf(T::new(&self.bounds.rect));
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

#[derive(PartialEq, Debug)]
pub enum LocationResult<T> {
    Found(T),
    Closest(T),
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::IRect;
    use wasm_bindgen_test::*;

    use crate::quadtree::{
        FromBounds, Node, Quad,
        util::{MIRect, Quadrant},
    };

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
            (MIRect::new(100, 100, 200, 200), IVec2::new(100, 100)),
            (MIRect::new(100, 100, 200, 200), IVec2::new(199, 199)),
            (MIRect::new(-100, -100, 0, 0), IVec2::new(-100, -100)),
            (MIRect::new(-100, -100, 0, 0), IVec2::new(-1, -1)),
        ];
        for (bounds, point) in testbounds {
            let mut quad = Quad::<Dummy>::from_bounds(bounds);
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

    // ===== Quad Tree Creation & Basic Properties =====

    #[wasm_bindgen_test]
    fn quad_frombounds() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let quad = Quad::from_bounds(bounds);

        assert!(quad.is_leaf());
        assert_eq!(quad.get_bounds(), &bounds.rect);
        assert_eq!(quad.get_leaf(), Some(&Dummy::new(&bounds.rect)));
    }

    #[wasm_bindgen_test]
    fn quad_set_leaf() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));

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
    fn quad_set_min_depth_zero() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));
        let point = IVec2::new(10, 10);

        // Setting min depth 0 should return self (root)
        let result = quad.set_min_depth(&point, 0);
        assert!(std::ptr::eq(result, &quad));
        assert!(quad.is_leaf()); // Should remain leaf at depth 0
    }

    #[wasm_bindgen_test]
    fn test_set_min_depth_multiple_levels() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));
        let point = IVec2::new(-25, 25); // Top-left quadrant

        // Set depth to 2 (creates 2 levels of subdivision)
        let _parent = quad.set_min_depth(&point, 2);

        // Verify the hierarchy was created correctly
        if let Node::Branch(children) = &quad.node {
            // First level - top-left quadrant should be branched
            let top_left = &children[0];
            assert!(!top_left.is_leaf());

            if let Node::Branch(grand_children) = &top_left.node {
                // Second level - should find our target point's quadrant
                let target_quadrant = Quadrant::from_point(&top_left.bounds, &point).unwrap();
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
    fn quad_set_min_depth_edge() {
        // Test points exactly on quadrant boundaries
        let edge_points = [
            IVec2::new(0, 0),   // Exact center
            IVec2::new(-50, 0), // Left edge
            IVec2::new(0, -50), // Bottom edge
        ];

        for point in edge_points {
            let mut quad = Quad::<Dummy>::new(IVec2::splat(100));

            let _result = quad.set_min_depth(&point, 1);
            assert!(
                !quad.is_leaf(),
                "Should subdivide for edge point {:?}",
                point
            );
        }
    }

    #[wasm_bindgen_test]
    fn test_set_max_depth_merging() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));
        let point = IVec2::new(25, 25);

        // First create some depth
        quad.set_min_depth(&point, 2);
        assert!(!quad.is_leaf());

        // Then set max depth to 1 (should merge back one level)
        quad.set_max_depth(&point, 1);

        // Root should still be branched (depth 1)
        assert!(!quad.is_leaf());

        // But the direct child containing our point should now be a leaf
        if let Node::Branch(children) = &quad.node {
            let target_quadrant = Quadrant::from_point(&quad.bounds, &point).unwrap();
            assert!(children[target_quadrant as usize].is_leaf());
        }
    }

    // ===== Bounds Matching Tests =====

    #[wasm_bindgen_test]
    fn test_matching_bounds_exact_match() {
        let bounds = MIRect::new(0, 0, 100, 100);
        let mut quad = Quad::<Dummy>::from_bounds(bounds);

        let result = quad.matching_bounds(&bounds.rect);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &bounds.rect);
        }
    }

    #[wasm_bindgen_test]
    fn test_matching_bounds_partial_overlap() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));
        let key = 42;
        quad.set_leaf(Dummy(key));

        // Bounds that cross multiple quadrants
        let crossing_bounds = IRect::new(-25, -25, 0, 0);
        let result = quad.matching_bounds(&crossing_bounds);
        let LocationResult::Closest(Quad {
            node: Node::Leaf(k),
            ..
        }) = result
        else {
            panic!("Expected Closest, got {:?}", result);
        };
        assert_eq!(k.0, key);
    }

    #[wasm_bindgen_test]
    fn quad_matching_bounds() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));

        // Subdivide first
        quad.subdivide();

        // Test bounds that fit entirely in one quadrant
        let top_left_bounds = IRect::new(-50, 0, 0, 50);
        let result = quad.matching_bounds(&top_left_bounds);
        assert!(matches!(result, LocationResult::Found(_)));

        if let LocationResult::Found(found) = result {
            assert_eq!(found.get_bounds(), &top_left_bounds);
        }
    }

    #[wasm_bindgen_test]
    fn quad_matching_bounds_outside() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));

        // Bounds completely outside
        let outside_bounds = IRect::new(200, 200, 300, 300);
        let result = quad.matching_bounds(&outside_bounds);
        assert!(matches!(result, LocationResult::NotFound));
    }

    // ===== Traversal Tests =====

    #[wasm_bindgen_test]
    fn quad_for_each_single() {
        let quad = Quad::<Dummy>::new(IVec2::splat(100));
        let mut count = 0;

        quad.for_each(&mut |_| count += 1);

        assert_eq!(count, 1);
    }

    // ===== Edge Cases =====

    #[wasm_bindgen_test]
    #[should_panic(expected = "assertion failed")]
    fn quad_subdivide_error() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(1));

        quad.set_min_depth(&IVec2::new(0, 0), 1);
    }

    #[wasm_bindgen_test]
    fn test_deep_nesting() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(128));
        let point = IVec2::new(-64, 63); // Near top-left corner

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
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));

        // Convert to branch
        quad.subdivide();

        // This should panic because we're a branch, not a leaf
        quad.set_leaf(Dummy(42));
    }

    #[wasm_bindgen_test]
    fn test_quad_parent_child_contains() {
        let mut quad = Quad::<Dummy>::new(IVec2::splat(100));
        quad.subdivide();

        for x in quad.bounds.rect.min.x - 1..quad.bounds.rect.max.x + 1 {
            for y in quad.bounds.rect.min.y - 1..quad.bounds.rect.max.y + 1 {
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
