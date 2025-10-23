use bevy::math::{IRect, IVec2};

use crate::contains;

pub struct Quad<T> {
    data: Node<T>,
    bounds: IRect,
}

enum Node<T> {
    Branch([Box<Quad<T>>; 4]),
    Leaf(T),
}

impl<T> Quad<T> {
    pub fn new(bounds: IRect, value: T) -> Self {
        Self {
            data: Node::Leaf(value),
            bounds: bounds,
        }
    }

    pub fn smallest_containing(&self, point: &IVec2) -> Option<&Quad<T>> {
        let Node::Branch(children) = &self.data else {
            return Some(self);
        };

        debug_assert!(self.bounds.contains(*point));

        let center = self.bounds.center();

        let is_left = center.x < point.x;
        let is_bottom = center.y < point.y;

        // Top Right = 0, Bottom Right = 1, Top Left = 2, Bottom Left = 3
        Some(&children[is_left as usize * 2 + is_bottom as usize])
    }

    /// Returns the smallest child quad that fully contains the given rect.
    pub fn smallest_containing_rect(&self, rect: &IRect) -> Result<&Quad<T>, &'static str> {
        if !contains(&self.bounds, rect) {
            return Err("Quad does not contain rect.");
        }
        if let Node::Branch(children) = &self.data {
            for child in children {
                if contains(&child.bounds, rect) {
                    return Ok(&child);
                }
            }
        }
        Ok(self)
    }
    
    pub fn smallest_containing_rect_mut(&mut self, rect: &IRect) -> Result<&mut Quad<T>, &'static str> {
        if !contains(&self.bounds, rect) {
            return Err("Quad does not contain rect.");
        }
        
        if let Node::Branch(children) = &mut self.data {
            for child in children {
                if contains(&child.bounds, rect) {
                    return Ok(child);
                }
            }
        }
        Ok(self)
    }

    /// Turns this node into a leaf
    pub fn unite<F: FnMut(&IRect) -> T>(&mut self, mut builder: F) {
        debug_assert!(self.is_branch());
        self.data = Node::Leaf(builder(&self.bounds));
    }

    /// Turns this node into a branch
    pub fn subdivide<F: FnMut(&IRect) -> T>(&mut self, mut builder: F) {
        debug_assert!(self.is_leaf());

        let bounds = self.bounds;
        let center = bounds.center();

        let top_right = IRect {
            min: center,
            max: bounds.max,
        };
        let bottom_right = IRect::new(center.x, bounds.min.y, bounds.max.x, center.y);
        let top_left = IRect::new(bounds.min.x, center.y, center.x, bounds.max.y);
        let bottom_left = IRect {
            min: bounds.min,
            max: center,
        };

        let quarters = [top_right, bottom_right, top_left, bottom_left];
        self.data =
            Node::Branch(quarters.map(|quarter| Box::new(Quad::new(quarter, builder(&quarter)))));
    }

    pub fn is_branch(&self) -> bool {
        matches!(self.data, Node::Branch(_))
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self.data, Node::Leaf(_))
    }

    pub fn get_bounds(&self) -> &IRect {
        &self.bounds
    }
}