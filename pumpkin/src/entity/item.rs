use std::sync::{Arc, atomic::AtomicU32};
use std::sync::atomic::Ordering;
use async_trait::async_trait;
use pumpkin_data::{damage::DamageType, item::Item};
use pumpkin_protocol::{
    client::play::{CTakeItemEntity, MetaDataType, Metadata},
    codec::item_stack_seralizer::ItemStackSerializer,
};
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::item::ItemStack;
use tokio::sync::Mutex;
use pumpkin_data::entity::EntityType;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use crate::server::Server;

use super::{Entity, EntityBase, living::LivingEntity, player::Player};

const DEFAULT_ITEM_DESPAWN_RATE: u32 = 6000;
const DEFAULT_ITEM_MERGE_RADIUS: f64 = 0.5;

pub struct ItemEntity {
    entity: Entity,
    item_age: AtomicU32,
    // These cannot be atomic values because we mutate their state based on what they are; we run
    // into the ABA problem
    item_stack: Mutex<ItemStack>,
    pickup_delay: Mutex<u16>,
}

impl ItemEntity {
    pub async fn new(entity: Entity, item_id: u16, count: u32) -> Self {
        /*entity
            .set_velocity(Vector3::new(
                rand::random::<f64>() * 0.2 - 0.1,
                0.2,
                rand::random::<f64>() * 0.2 - 0.1,
            ))
            .await;*/
        entity.yaw.store(rand::random::<f32>() * 360.0);
        Self {
            entity,
            item_stack: Mutex::new(ItemStack::new(
                count as u8,
                Item::from_id(item_id).expect("We passed a bad item id into ItemEntity"),
            )),
            item_age: AtomicU32::new(0),
            pickup_delay: Mutex::new(40), // Vanilla pickup delay is 40 ticks
        }
    }
    pub async fn send_meta_packet(&self) {
        self.entity
            .send_meta_data(&[Metadata::new(
                8,
                MetaDataType::ItemStack,
                &ItemStackSerializer::from(self.item_stack.lock().await.clone()),
            )])
            .await;
    }
    
    pub async fn set_pickup_delay(&self, delay: u16) {
        let mut lock = self.pickup_delay.lock().await;
        *lock = delay;
    }

    async fn set_underwater_movement(&self) {
        self.set_fluid_movement(0.99).await;
    }

    async fn set_lava_movement(&self) {
        self.set_fluid_movement(0.95).await;
    }

    async fn set_fluid_movement(&self, multiplier: f64) {
        let mut velocity = self.entity.velocity.load();
        velocity.x *= multiplier;
        velocity.y += if velocity.y < 0.06 {
            5.0E-4
        } else {
            0.0
        };
        velocity.z *= multiplier;
        self.entity.set_velocity(velocity).await;
    }
    
    pub async fn is_mergable(&self) -> bool {
        let item_stack = self.item_stack.lock().await;
        let pickup_delay = self.pickup_delay.lock().await;
        *pickup_delay != 32767 && self.item_age.load(Ordering::Relaxed) < DEFAULT_ITEM_DESPAWN_RATE && item_stack.item_count < item_stack.item.components.max_stack_size
    }
    
    async fn merge_with_neighbours(&self) {
        let radius = DEFAULT_ITEM_MERGE_RADIUS;
        
        let current_pos = self.entity.pos.load();
        let search_box = BoundingBox::new(
            Vector3::new(current_pos.x - radius, current_pos.y - radius, current_pos.z - radius),
            Vector3::new(current_pos.x + radius, current_pos.y + radius, current_pos.z + radius),
        );
        
        let world = self.entity.world.read().await;
        let nearby_entities = world.get_entities_in_box_of_type::<ItemEntity>(&search_box, EntityType::ITEM).await;
        
        let mut current_stack = self.item_stack.lock().await;
        
        for item_entity in nearby_entities {
            if item_entity.get_entity().entity_id == self.entity.entity_id {
                continue;
            }
            if !item_entity.is_mergable().await {
                continue;
            }
            let mut other_stack = item_entity.item_stack.lock().await;
            if other_stack.item.id != current_stack.item.id {
                continue;
            }
            
            let max_stack_size = current_stack.item.components.max_stack_size;
            let space_left = max_stack_size - current_stack.item_count;
            
            if space_left <= 0 {
                continue;
            }
            
            let transfer_amount = std::cmp::min(other_stack.item_count, space_left);
            current_stack.item_count += transfer_amount;
            other_stack.item_count -= transfer_amount;
            
            self.send_meta_packet().await;
            
            if other_stack.item_count == 0 {
                item_entity.entity.remove().await;
            } else {
                item_entity.send_meta_packet().await;
            }
        }
    }
}

#[async_trait]
impl EntityBase for ItemEntity {
    async fn tick(&self, server: &Server) {
        // TODO: Uncomment this when entity tick is implemented
        //self.entity.tick(server).await;
        {
            let mut delay = self.pickup_delay.lock().await;
            // 32767 in Vanilla is implemented as "no delay"
            if *delay > 0 && *delay != 32767 {
                *delay = delay.saturating_sub(1);
            }
        };

        // TODO: Check fluid height > 0.1
        if self.entity.is_in_water().await {
            self.set_underwater_movement().await;
        } else if self.entity.is_in_lava().await {
            self.set_lava_movement().await;
        } else {
            self.entity.apply_gravity().await;
        }

        // Check for no physics

        let on_ground = self.entity.on_ground.load(Ordering::Relaxed);
        let velocity = self.entity.velocity.load();
        let tick_count = self.entity.tick_count.load(Ordering::Relaxed);
        let world = self.entity.world.read().await;
        
        if !on_ground || velocity.horizontal_length_squared() > 1.0E-5 || (tick_count + self.entity.entity_id) % 4 == 0{
            // slipperiness is friction of the block
            self.entity.tick_move(self.entity.velocity.load()).await;
            
            let mut f = 0.98;
            
            if on_ground {
                if let Ok(block) = world.get_block(&self.get_block_pos_below_that_affects_my_movement().await).await {
                    f = block.slipperiness as f64 * 0.98;
                } else {
                    return;
                }
            }
            
            let velocity = self.entity.velocity.load();
            self.entity.set_velocity(velocity.multiply_raw(f, 0.98, f)).await;
            
            if on_ground {
                let velocity = self.entity.velocity.load();
                if velocity.y < 0.0 {
                    self.entity.set_velocity(velocity.multiply_raw(1.0, -0.5, 1.0)).await;
                }
            }
        }
        
        if tick_count % 40 == 0 && self.is_mergable().await {
            self.merge_with_neighbours().await;
        }
        
        println!("Position: {:?}", self.entity.pos.load());

        let age = self
            .item_age
            .fetch_add(1, Ordering::Relaxed);
        if age >= DEFAULT_ITEM_DESPAWN_RATE {
            self.entity.remove().await;
        }
    }
    async fn damage(&self, _amount: f32, _damage_type: DamageType) -> bool {
        false
    }

    fn get_default_gravity(&self) -> f64 {
        0.04
    }

    async fn get_block_pos_below_that_affects_my_movement(&self) -> BlockPos {
        self.entity.get_on_pos(0.999999).await
    }

    async fn on_player_collision(&self, player: Arc<Player>) {
        let can_pickup = {
            let delay = self.pickup_delay.lock().await;
            *delay == 0
        };

        if can_pickup {
            let mut inv = player.inventory.lock().await;
            let mut total_pick_up = 0;
            let mut slot_updates = Vec::new();
            let remove_entity = {
                let item_stack = self.item_stack.lock().await.clone();
                let mut stack_size = item_stack.item_count;
                let max_stack = item_stack.item.components.max_stack_size;
                while stack_size > 0 {
                    if let Some(slot) = inv.get_pickup_item_slot(item_stack.item.id) {
                        // Fill the inventory while there are items in the stack and space in the inventory
                        let maybe_stack = inv
                            .get_slot(slot)
                            .expect("collect item slot returned an invalid slot");

                        if let Some(existing_stack) = maybe_stack {
                            // We have the item in this stack already

                            // This is bounded to `u8::MAX`
                            let amount_to_fill = u32::from(max_stack - existing_stack.item_count);
                            // This is also bounded to `u8::MAX` since `amount_to_fill` is max `u8::MAX`
                            let amount_to_add = amount_to_fill.min(u32::from(stack_size));
                            // Therefore this is safe

                            // Update referenced stack so next call to `get_pickup_item_slot` is
                            // correct
                            existing_stack.item_count += amount_to_add as u8;
                            total_pick_up += amount_to_add;

                            debug_assert!(amount_to_add > 0);
                            stack_size = stack_size.saturating_sub(amount_to_add as u8);

                            slot_updates.push((slot, existing_stack.clone()));
                        } else {
                            // A new stack

                            // This is bounded to `u8::MAX`
                            let amount_to_fill = u32::from(max_stack);
                            // This is also bounded to `u8::MAX` since `amount_to_fill` is max `u8::MAX`
                            let amount_to_add = amount_to_fill.min(u32::from(stack_size));
                            total_pick_up += amount_to_add;

                            debug_assert!(amount_to_add > 0);
                            stack_size = stack_size.saturating_sub(amount_to_add as u8);

                            slot_updates.push((slot, self.item_stack.lock().await.clone()));
                        }
                    } else {
                        // We can't pick anything else up
                        break;
                    }
                }

                stack_size == 0
            };

            if total_pick_up > 0 {
                player
                    .client
                    .enqueue_packet(&CTakeItemEntity::new(
                        self.entity.entity_id.into(),
                        player.entity_id().into(),
                        total_pick_up.try_into().unwrap(),
                    ))
                    .await;
            }

            // TODO: Can we batch slot updates?
            for (slot, stack) in slot_updates {
                player.update_single_slot(&mut inv, slot, stack).await;
            }

            if remove_entity {
                self.entity.remove().await;
            } else {
                // Update entity
                self.send_meta_packet().await;
            }
        }
    }

    fn get_entity(&self) -> &Entity {
        &self.entity
    }

    fn get_living_entity(&self) -> Option<&LivingEntity> {
        None
    }
}
