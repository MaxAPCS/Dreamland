use avian3d::prelude::{Collider, RigidBody};
use bevy::ecs::resource::Resource;
use bevy::prelude::*;

use crate::compute::{mesh, simplexcpu};
use crate::player;
use crate::quadtree::Quad;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_world)
            .add_systems(FixedPreUpdate, generate_chunks);
    }
}

const SMALLEST_CHUNK: i32 = 128;

#[derive(Resource)]
struct World {
    chunks: Quad<Entity>,
}

fn create_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bounds = IRect::from_center_size(IVec2::ZERO, IVec2::splat(4096));
    let chunk = spawn_chunk(&mut commands, &asset_server, &bounds);
    commands.insert_resource(World {
        chunks: Quad::new(bounds, chunk),
    });
}

fn generate_chunks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut world: ResMut<World>,
    player: Single<&Transform, (Changed<Transform>, With<player::Player>)>,
) {
    let player = IVec2 {
        x: player.translation.x as i32,
        y: player.translation.z as i32,
    };

    let mut bounds = IRect::from_center_size(player, IVec2::splat(1024));
    let mut smallest: &mut Quad<Entity> = &mut world.chunks;

    // FIXME - this logic is flawed lmao
    while bounds.width() >= SMALLEST_CHUNK {
        smallest.unite(|&quarter: &IRect| spawn_chunk(&mut commands, &asset_server, &quarter));

        smallest = smallest.smallest_containing_rect_mut(&bounds).unwrap();

        if smallest.get_bounds().width() > bounds.width() {
            smallest
                .subdivide(|&quarter: &IRect| spawn_chunk(&mut commands, &asset_server, &quarter));
            bounds = IRect::from_center_size(player, bounds.half_size());
        }
    }

    /*
    Ideally, actions on chunks should be deferred until the end of the update.
    This way:
    - if one chunk is subdivided, it should not try to generate its former Leaf node self.
    - if one chunk is united, it should not regenerate its Leaf node self if it will be subdivided later.
    */
}

fn spawn_chunk(commands: &mut Commands, asset_server: &Res<AssetServer>, bounds: &IRect) -> Entity {
    let mesh = mesh::mesh(
        mesh::Chunk {
            x: bounds.min.x,
            z: bounds.min.y,
            w: bounds.width() as usize,
            h: bounds.height() as usize,
            lod: (bounds.width() / 16) as usize,
        },
        &|pos| (simplexcpu::fbm_simplex_2d_seeded(pos, 4, 2., 0.5, 0.) * 4.).exp2(),
    );
    // Only the smallest chunks get physics.
    if bounds.width() >= SMALLEST_CHUNK {
        commands
            .spawn((
                Mesh3d(asset_server.add(mesh)),
                MeshMaterial3d(asset_server.add(StandardMaterial {
                    base_color: Color::linear_rgb(0., 1., 0.),
                    ..Default::default()
                })),
                Transform::from_translation(Vec3 {
                    x: bounds.min.x as f32,
                    y: 0.,
                    z: bounds.min.y as f32,
                }),
            ))
            .id()
    } else {
        commands
            .spawn((
                RigidBody::Static,
                Collider::trimesh_from_mesh(&mesh).unwrap(),
                Mesh3d(asset_server.add(mesh)),
                MeshMaterial3d(asset_server.add(StandardMaterial {
                    base_color: Color::linear_rgb(0., 1., 0.),
                    ..Default::default()
                })),
                Transform::from_translation(Vec3 {
                    x: bounds.min.x as f32,
                    y: 0.,
                    z: bounds.min.y as f32,
                }),
            ))
            .id()
    }
}
