use crate::math::boundingbox::BoundingBox;
use crate::math::vector3::Vector3;

#[derive(Debug, Clone)]
pub struct VoxelShape {
    pub boxes: Vec<BoundingBox>,
    pub collision_type: CollisionType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionType {
    /// No collision
    Empty,
    /// Partial collision
    Partial,
    /// Full collision
    Full,
}

impl VoxelShape {
    
    pub fn new(bbox: BoundingBox) -> Self {
        Self {
            boxes: vec![bbox],
            collision_type: CollisionType::Full,
        }
    }

    pub fn from_boxes(boxes: Vec<BoundingBox>) -> Self {
        if boxes.is_empty() {
            return Self::empty();
        }
        Self {
            boxes,
            collision_type: CollisionType::Partial,
        }
    }

    pub fn empty() -> Self {
        Self {
            boxes: Vec::new(),
            collision_type: CollisionType::Empty,
        }
    }

    pub fn cube(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self::new(BoundingBox::new(Vector3::new(min_x, min_y, min_z), Vector3::new(max_x, max_y, max_z)))
    }

    pub fn collides_with(&self, bbox: &BoundingBox) -> bool {
        self.boxes.iter().any(|shape_box| shape_box.intersects(bbox))
    }

    // Calculate the resulting movement after a collision with this shape
    pub fn calculate_collision_offset(&self, entity_box: &BoundingBox, movement: Vector3<f64>) -> Vector3<f64> {
        if self.boxes.is_empty() || movement.length_squared() < 1.0E-7 {
            return movement;
        }

        let mut result = movement.clone();

        // Verify the collision on the X axis
        if movement.x != 0.0 {
            let x_offset_box = entity_box.expand_towards(movement.x, 0.0, 0.0);
            for box_shape in &self.boxes {
                if box_shape.intersects(&x_offset_box) {
                    if movement.x > 0.0 {
                        result.x = result.x.min(box_shape.min.x - entity_box.max.x);
                    } else {
                        result.x = result.x.max(box_shape.max.x - entity_box.min.x);
                    }

                    if result.x.abs() < 1.0E-7 {
                        result.x = 0.0;
                    }
                }
            }
        }

        // Then verify the collision on the Y axis
        if movement.y != 0.0 {
            let moved_x_box = entity_box.offset_raw(result.x, 0.0, 0.0);
            let y_offset_box = moved_x_box.expand_towards(0.0, movement.y, 0.0);

            for box_shape in &self.boxes {
                if box_shape.intersects(&y_offset_box) {
                    if movement.y > 0.0 {
                        result.y = result.y.min(box_shape.min.y - moved_x_box.max.y);
                    } else {
                        result.y = result.y.max(box_shape.max.y - moved_x_box.min.y);
                    }

                    if result.y.abs() < 1.0E-7 {
                        result.y = 0.0;
                    }
                }
            }
        }

        // Finally, verify the collision on the Z axis
        if movement.z != 0.0 {
            let moved_xy_box = entity_box.offset_raw(result.x, result.y, 0.0); // TODO: implement offset
            let z_offset_box = moved_xy_box.expand_towards(0.0, 0.0, movement.z);

            for box_shape in &self.boxes {
                if box_shape.intersects(&z_offset_box) {
                    if movement.z > 0.0 {
                        result.z = result.z.min(box_shape.min.z - moved_xy_box.max.z);
                    } else {
                        result.z = result.z.max(box_shape.max.z - moved_xy_box.min.z);
                    }

                    if result.z.abs() < 1.0E-7 {
                        result.z = 0.0;
                    }
                }
            }
        }

        result
    }


    /// Creates a cubic VoxelShape that cover the entire block
    pub fn full_cube() -> Self {
        Self::cube(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)
    }


    pub fn union(&self, other: &VoxelShape) -> Self {
        let mut boxes = self.boxes.clone();
        boxes.extend(other.boxes.clone());
        Self::from_boxes(boxes)
    }


    pub fn is_empty(&self) -> bool {
        self.collision_type == CollisionType::Empty
    }
}
