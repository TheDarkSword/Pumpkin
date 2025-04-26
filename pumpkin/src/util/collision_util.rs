use std::sync::Arc;
use bitflags::bitflags;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::math::voxel_shape::VoxelShape;
use crate::world::World;

pub struct CollisionUtil;

impl CollisionUtil {

    /// dy < 0
    pub fn cut_dawnwards(bounding_box: &BoundingBox, dy: f64) -> BoundingBox {
        BoundingBox::new(
            Vector3::new(
                bounding_box.min.x,
                bounding_box.min.y + dy, // dy is negative
                bounding_box.min.z,
            ),
            Vector3::new(
                bounding_box.max.x,
                bounding_box.min.y,
                bounding_box.max.z,
            ),
        )
    }

    /// dy > 0
    pub fn cut_upwards(bounding_box: &BoundingBox, dy: f64) -> BoundingBox {
        BoundingBox::new(
            Vector3::new(
                bounding_box.min.x,
                bounding_box.max.y,
                bounding_box.min.z,
            ),
            Vector3::new(
                bounding_box.max.x,
                bounding_box.max.y + dy, // dy is positive
                bounding_box.max.z,
            ),
        )
    }

    pub async fn get_potential_shapes(
        world: &Arc<World>,
        entity_box: &BoundingBox,
        movement: &Vector3<f64>,
    ) -> Vec<(BlockPos, VoxelShape)> {
        let expanded_box = entity_box.expand_towards(movement.x, movement.y, movement.z);
        let min_block_x = (expanded_box.min.x - 1.0).floor() as i32;
        let min_block_y = (expanded_box.min.y - 1.0).floor() as i32;
        let min_block_z = (expanded_box.min.z - 1.0).floor() as i32;
        let max_block_x = (expanded_box.max.x + 1.0).ceil() as i32;
        let max_block_y = (expanded_box.max.y + 1.0).ceil() as i32;
        let max_block_z = (expanded_box.max.z + 1.0).ceil() as i32;

        let mut shapes = Vec::new();

        for y in min_block_y..max_block_y {
            for z in min_block_z..max_block_z {
                for x in min_block_x..max_block_x {
                    let pos = BlockPos(Vector3::new(x, y, z));
                    if let Ok(block_state) = world.get_block_state(&pos).await {
                        
                        if block_state.is_solid {
                            // If there aren't any collision shapes, we can add a full cube
                            if block_state.collision_shapes.is_empty() {
                                shapes.push((pos, VoxelShape::full_cube()));
                            } else {
                                let mut collision_boxes = Vec::new();

                                if let Some(collision_shapes) = pumpkin_data::block::get_block_collision_shapes(block_state.id) {
                                    for shape in collision_shapes {
                                        collision_boxes.push(BoundingBox::new_array(shape.min, shape.max))
                                    }
                                }
                                
                                if !collision_boxes.is_empty() {
                                    shapes.push((pos, VoxelShape::from_boxes(collision_boxes)));
                                }
                            }
                        }
                    }
                }
            }
        }

        shapes
    }


    pub async fn get_entity_collisions(
        world: &Arc<World>,
        entity_id: i32,
        entity_box: &BoundingBox,
    ) -> Vec<BoundingBox> {
        let entities = world.get_entities_in_box(entity_box).await;
        let mut boxes = Vec::new();

        for entity in entities {
            if entity.get_entity().entity_id != entity_id {
                boxes.push(entity.get_entity().bounding_box.load());
            }
        }

        boxes
    }

}

bitflags! {
    pub struct CollisionFlags: u8 {
        const LOAD_CHUNKS = 1 << 0;
        const COLLIDE_WITH_UNLOADED_CHUNKS = 1 << 1;
        const CHECK_BORDER = 1 << 2;
        const CHECK_ONLY = 1 << 3;
    }
}