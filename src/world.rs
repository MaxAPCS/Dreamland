use bevy::prelude::*;

use crate::{
    compute::{self, GenerationMessage},
    player::Player,
    quadtree::*,
};
pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chunks>()
            .add_systems(FixedPreUpdate, chunk_link.after(compute::GenerationWork))
            .add_systems(FixedPostUpdate, (chunk_initialize, chunk_generate).chain());
    }
}

#[derive(Resource)]
struct Chunks(Quad<Chunk>);

#[derive(Debug)]
enum Chunk {
    Pending(IRect),
    Exists,
    Found(Entity),
}

const WORLD_SIZE: u32 = 2048;
const MAX_DEPTH: usize = 4;
const LEVELS: [u32; MAX_DEPTH] = {
    let mut depth = 0;
    let mut out = [0u32; MAX_DEPTH];
    loop {
        out[depth] = 128u32 * 2u32.pow((MAX_DEPTH - 1 - depth) as u32);
        depth += 1;
        if depth == MAX_DEPTH {
            break;
        }
    }
    out
};

// Initialization
impl Default for Chunks {
    fn default() -> Self {
        Self(Quad::new(IRect::from_center_size(
            IVec2::ZERO,
            IVec2::splat(WORLD_SIZE as i32),
        )))
    }
}
impl FromBounds for Chunk {
    fn new(bounds: &IRect) -> Self {
        Chunk::Pending(bounds.clone())
    }
}

fn chunk_initialize(
    mut world: ResMut<Chunks>,
    player_query: Query<&Transform, (With<Player>, Changed<Transform>)>,
) {
    let Ok(Transform {
        translation: player_pos,
        ..
    }) = player_query.single()
    else {
        return;
    };

    let player_pos = player_pos.xz().as_ivec2();
    for (depth, &size) in LEVELS.iter().enumerate() {
        let rect = IRect::from_center_size(player_pos, IVec2::splat(size as i32));
        for corner in &[
            rect.min,
            IVec2::new(rect.max.x, rect.min.y),
            rect.max,
            IVec2::new(rect.min.x, rect.max.y),
        ] {
            world.0.set_min_depth(corner, depth as u32 + 1);
        }
    }
}

// Creation
impl Quad<Chunk> {
    fn generate(&mut self, inv_queue: &mut ResMut<compute::InvocationQueue>) {
        if let Some(Chunk::Pending(bounds)) = self.get_leaf() {
            bevy::log::debug!("GENER {}", self);
            inv_queue.add(bounds.to_owned());
            self.set_leaf(Chunk::Exists);
        }
        // does nothing if not pending
    }
}
fn chunk_generate(mut world: ResMut<Chunks>, mut inv_queue: ResMut<compute::InvocationQueue>) {
    world
        .0
        .for_each(&mut |chunk| chunk.generate(&mut inv_queue));
}

// Linking
impl Chunks {
    // find the chunk matching these bounds, and assign the id to that chunk (exists -> found)
    fn link(&mut self, bounds: &IRect, id: Entity) -> Result<(), ()> {
        match self.0.matching_bounds(bounds) {
            LocationResult::Found(chunk) => {
                bevy::log::debug!("LINKD {}", chunk);
                chunk.link(id);
                Ok(())
            }
            LocationResult::Closest(_) => unreachable!(), // This indicates a build was queued for a branch- but only leaves queue builds.
            LocationResult::NotFound => Err(()),          // orphaned chunk, destroy
        }
    }
}
impl Quad<Chunk> {
    fn link(&mut self, id: Entity) {
        debug_assert!(
            matches!(self.get_leaf().expect("Node Should Be Leaf"), Chunk::Exists),
            "Node State Exception: Expected [Exists], Found [{:?}]",
            self.get_leaf().unwrap()
        );
        self.set_leaf(Chunk::Found(id));
    }
}
fn chunk_link(
    mut world: ResMut<Chunks>,
    mut messager: MessageReader<compute::GenerationMessage>,
    mut commands: Commands,
) {
    for GenerationMessage { bounds, entity } in messager.read() {
        if let Err(_) = world.link(bounds, entity.clone()) {
            debug_assert!(false, "Orphaned Chunk Detected");
            commands.get_entity(*entity).unwrap().despawn();
        }
    }
}

// Destruction
impl Drop for Chunk {
    fn drop(&mut self) {
        if let Chunk::Found(_) = self {
            // delete from the world
            bevy::log::info!("Orphaned Chunk Created");
        }
    }
}
