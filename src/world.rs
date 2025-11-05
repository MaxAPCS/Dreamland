use bevy::prelude::*;

use crate::compute;
pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_world);
    }
}

fn create_world(mut inv_queue: ResMut<compute::InvocationQueue>) {
    inv_queue.add(IRect::new(0, 0, 128, 128));
    inv_queue.add(IRect::new(-128, 0, 0, 128));
    inv_queue.add(IRect::new(-128, -128, 0, 0));
    inv_queue.add(IRect::new(0, -128, 128, 0));
    inv_queue.add(IRect::new(128, -128, 128 + 256, 128)); // right
    inv_queue.add(IRect::new(-128, 128, 128, 128 + 256)); // top
    inv_queue.add(IRect::new(-128 - 256, -128, -128, 128)); // left
    inv_queue.add(IRect::new(-128, -128 - 256, 128, -128)); // bottom
}
