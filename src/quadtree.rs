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

    pub fn set_min_depth(&mut self, point: &IVec2, target_depth: u32) {
        self.set_min_depth_recurse(point, target_depth, 0)
    }

    fn set_min_depth_recurse(&mut self, point: &IVec2, target_depth: u32, current_depth: u32) {
        debug_assert!(self.bounds.contains(*point));

        if current_depth >= target_depth {
            return;
        }

        if self.is_leaf() {
            self.subdivide();
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
