use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use bevy::render::RenderStartup;

use crate::compute;
use crate::compute::ComputeInit;
use crate::cpu;
pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_base);
        app.sub_app_mut(bevy::render::RenderApp)
            .add_systems(RenderStartup, create_world.after(ComputeInit));
        // .add_systems(FixedPreUpdate, generate_chunks);
    }
}

// #[derive(Resource)]
// struct World {
//     chunks: Mutex<Quad<Entity>>,
// }

fn create_base(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn_chunk(&mut commands, &asset_server, IRect::new(0, 0, 128, 128));
    spawn_chunk(&mut commands, &asset_server, IRect::new(-128, 0, 0, 128));
    spawn_chunk(&mut commands, &asset_server, IRect::new(-128, -128, 0, 0));
    spawn_chunk(&mut commands, &asset_server, IRect::new(0, -128, 128, 0));
}

fn create_world(mut gcq: ResMut<compute::GlobalComputeQueue>) {
    gcq.add(IRect::new(128, -128, 128 + 256, 128));

    // commands.insert_resource(World {
    //     chunks: Mutex::new(Quad::new(bounds, chunk)),
    // });
}

// fn generate_chunks(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     world: Res<World>,
//     player: Single<&Transform, (Changed<Transform>, With<player::Player>)>,
// ) {
//     let player = IVec2 {
//         x: player.translation.x as i32,
//         y: player.translation.z as i32,
//     };

//     let mut bounds = IRect::from_center_size(player, IVec2::splat(1024));
//     let mut smallest: &Quad<Entity> = &world.chunks.lock().unwrap();

//     // FIXME - this logic is flawed lmao
//     while bounds.width() >= SMALLEST_CHUNK {
//         smallest.unite(|&quarter: &IRect| spawn_chunk(&mut commands, &asset_server, &quarter));

//         smallest = smallest.smallest_containing_rect(&bounds).unwrap();

//         if smallest.get_bounds().width() > bounds.width() {
//             smallest
//                 .subdivide(|&quarter: &IRect| spawn_chunk(&mut commands, &asset_server, &quarter));
//             bounds = IRect::from_center_size(player, bounds.half_size());
//         }
//     }

//     /*
//     Ideally, actions on chunks should be deferred until the end of the update.
//     This way:
//     - if one chunk is subdivided, it should not try to generate its former Leaf node self.
//     - if one chunk is united, it should not regenerate its Leaf node self if it will be subdivided later.
//     */
// }

fn spawn_chunk(commands: &mut Commands, asset_server: &Res<AssetServer>, bounds: IRect) {
    let (mesh, collider) = cpu::generation::generate_mesh_physics(&bounds, &|pos| {
        (cpu::invocation::fbm_simplex_2d_seeded(pos, 4, 2., 0.5, 0.) * 4.).exp2()
    });
    commands.spawn((
        RigidBody::Static,
        collider,
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
    ));
}
