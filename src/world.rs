use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use once_cell::sync::Lazy;
use tokio::sync::mpsc;

use crate::{compute, player::Player, quadtree::*};
pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chunks>()
            .init_resource::<PrevPlayerPos>()
            .add_systems(
                FixedPreUpdate,
                (chunk_destroy, chunk_link).after(compute::GenerationWork),
            )
            .add_systems(FixedPostUpdate, (chunk_initialize, chunk_generate).chain());
    }
}

#[derive(Resource, Default)]
struct PrevPlayerPos(Option<IVec2>);

#[derive(Resource)]
struct Chunks(Quad<Chunk>);

#[derive(Debug)]
enum Chunk {
    Empty,         // will not be created
    PreGen(IRect), // pre-generation
    PreLnk,        // pre-linking
    Real(Entity),  // linked, in the world
}

const WORLD_SIZE: u32 = 8192;
const MAX_DEPTH: usize = (WORLD_SIZE / 128).ilog2() as usize;
const LEVELS: [u32; MAX_DEPTH] = {
    let mut depth = 0;
    let mut out = [0u32; MAX_DEPTH];
    loop {
        out[depth] = 128u32 * (1u32 << (MAX_DEPTH - 1 - depth));
        depth += 1;
        if depth == MAX_DEPTH {
            break;
        }
    }
    out
};

// Initialization
impl FromBounds for Chunk {
    // -(initialize)-> Pending
    fn new(bounds: &IRect) -> Self {
        if bounds.width() >= 1024 as i32 {
            Chunk::Empty
        } else {
            Chunk::PreGen(bounds.clone())
        }
    }
}
impl Default for Chunks {
    fn default() -> Self {
        Self(Quad::new(IRect::from_center_size(
            IVec2::ZERO,
            IVec2::splat(WORLD_SIZE as i32),
        )))
    }
}
fn chunk_initialize(
    mut chunks: ResMut<Chunks>,
    player_query: Query<&Transform, (With<Player>, Changed<Transform>)>,
    mut prev_pos: ResMut<PrevPlayerPos>,
) {
    let Ok(Transform {
        translation: player_trans,
        ..
    }) = player_query.single()
    else {
        return;
    };

    let player_pos = player_trans.xz().as_ivec2();

    if let Some(player_pos_prev) = prev_pos.0 {
        if player_pos == player_pos_prev || player_pos.manhattan_distance(player_pos_prev) <= 16 {
            return;
        }
    }

    let player_pos_prev = *prev_pos.0.get_or_insert(player_pos);
    for (depth, &size) in LEVELS.iter().enumerate() {
        let size = IVec2::splat(size as i32);
        let rect = IRect::from_center_size(player_pos, size);
        let prevrect = IRect::from_center_size(player_pos_prev, size);

        let mut qualityzone = None;
        for corner in corners(&rect) {
            // If corner exceeds world bounds
            if !chunks.0.get_bounds().contains(corner) {
                continue;
            }
            let bounds = chunks
                .0
                .set_min_depth(&corner, depth as u32 + 1)
                .get_bounds();
            qualityzone = match qualityzone {
                None => Some(*bounds),
                Some(qualitybounds) => Some(qualitybounds.union(*bounds)),
            }
        }

        let qualityzone = qualityzone.unwrap();
        for corner in corners(&prevrect) {
            if !qualityzone.contains(corner) {
                chunks.0.set_max_depth(&corner, depth as u32);
            }
        }
    }

    if cfg!(debug_assertions) {
        bevy::log::debug!("{:?}", *chunks);
        bevy::log::debug!(
            "PLAYR ({}, {})",
            player_trans.x / 128.,
            player_trans.z / 128.
        );
    }
    prev_pos.0 = Some(player_pos);
}

// Creation
impl Quad<Chunk> {
    /// Pending -(generate)-> Exists
    fn generate(&mut self, inv_queue: &mut ResMut<compute::InvocationQueue>) {
        if let Some(Chunk::PreGen(bounds)) = self.get_leaf() {
            inv_queue.add(bounds.to_owned());
            self.set_leaf(Chunk::PreLnk);
        }
    }
}
fn chunk_generate(mut chunks: ResMut<Chunks>, mut inv_queue: ResMut<compute::InvocationQueue>) {
    chunks
        .0
        .for_each_mut(&mut |chunk| chunk.generate(&mut inv_queue));
}

// Linking
impl Quad<Chunk> {
    /// Exists -(link)-> Found
    fn link(&mut self, entity: Entity) {
        debug_assert!(
            matches!(self.get_leaf().expect("Node Should Be Leaf"), Chunk::PreLnk),
            "Node State Exception: Expected [Exists], Found [{:?}]",
            self.get_leaf().unwrap()
        );
        self.set_leaf(Chunk::Real(entity));
    }
}
impl Chunks {
    // find the chunk matching these bounds, and assign the id to that chunk (exists -> found)
    fn link(&mut self, bounds: &IRect, entity: Entity) -> Result<(), ()> {
        match self.0.matching_bounds(bounds) {
            LocationResult::Found(chunk) => {
                chunk.link(entity);
                Ok(())
            }
            LocationResult::Closest(_) => unreachable!(), // This indicates a build was queued for a branch- but only leaves queue builds.
            LocationResult::NotFound => Err(()),          // orphaned chunk, destroy
        }
    }
}
fn chunk_link(mut world: ResMut<Chunks>, mut messager: MessageReader<compute::GenerationMessage>) {
    for compute::GenerationMessage { bounds, entity } in messager.read() {
        if let Err(_) = world.link(bounds, entity.clone()) {
            bevy::log::warn!("Orphaned Chunk Detected");
            // Destroy the orphan.
            DESTROY_QUEUE.0.send(entity.clone()).unwrap();
        }
    }
}

// Destruction
static DESTROY_QUEUE: Lazy<(
    mpsc::UnboundedSender<Entity>,
    Arc<Mutex<mpsc::UnboundedReceiver<Entity>>>,
)> = Lazy::new(|| {
    let (s, r) = mpsc::unbounded_channel();
    (s, Arc::new(Mutex::new(r)))
});
impl Drop for Chunk {
    fn drop(&mut self) {
        if let Chunk::Real(entity) = self {
            DESTROY_QUEUE.0.send(entity.clone()).unwrap();
        }
    }
}
fn chunk_destroy(mut commands: Commands) {
    let mut recv = DESTROY_QUEUE.1.lock().unwrap();
    while let Ok(entity) = recv.try_recv() {
        commands.entity(entity).despawn();
    }
}

// Utility
fn corners(rect: &IRect) -> [IVec2; 4] {
    [
        rect.min,
        IVec2::new(rect.max.x, rect.min.y),
        rect.max,
        IVec2::new(rect.min.x, rect.max.y),
    ]
}
impl std::fmt::Debug for Chunks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut deb = f.debug_list();
        self.0.for_each(&mut |leaf| {
            deb.entry(leaf);
        });
        deb.finish()
    }
}
impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Chunk::Empty => "E",
            Chunk::PreGen(_) => "G",
            Chunk::PreLnk => "L",
            Chunk::Real(_) => "R",
        })
    }
}
