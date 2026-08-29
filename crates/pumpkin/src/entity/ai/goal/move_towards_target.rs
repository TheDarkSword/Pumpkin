use std::sync::Arc;

use super::{Controls, Goal};
use crate::entity::ai::util::default_random_pos;
use crate::entity::{EntityBase, ai::pathfinder::NavigatorGoal, mob::Mob};
use pumpkin_util::math::vector3::Vector3;

const HORIZONTAL_RANGE: i32 = 16;
const VERTICAL_RANGE: i32 = 7;

/// Mirrors vanilla Minecraft's `MoveTowardsTargetGoal`.
///
/// Moves the mob towards its current attack target within a specified maximum distance.
pub struct MoveTowardsTargetGoal {
    goal_control: Controls,
    speed: f64,
    within: f32,
    target: Option<Arc<dyn EntityBase>>,
    wanted_pos: Option<Vector3<f64>>,
}

impl MoveTowardsTargetGoal {
    #[must_use]
    pub const fn new(speed: f64, within: f32) -> Self {
        Self {
            goal_control: Controls::MOVE,
            speed,
            within,
            target: None,
            wanted_pos: None,
        }
    }
}

impl Goal for MoveTowardsTargetGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let target = mob.get_mob_entity().get_target();
        let Some(target) = target else {
            self.target = None;
            return false;
        };

        if !target.get_entity().is_alive() {
            self.target = None;
            return false;
        }

        let mob_pos = mob.get_entity().pos.load();
        let target_pos = target.get_entity().pos.load();
        let dist_sq = mob_pos.squared_distance_to_vec(&target_pos);
        let within_sq = f64::from(self.within) * f64::from(self.within);

        if dist_sq > within_sq {
            self.target = None;
            return false;
        }

        let pos = default_random_pos::get_pos_towards(
            mob,
            HORIZONTAL_RANGE,
            VERTICAL_RANGE,
            target_pos,
            std::f64::consts::FRAC_PI_2,
        );
        let Some(pos) = pos else {
            self.target = None;
            return false;
        };

        self.wanted_pos = Some(pos);
        self.target = Some(target);
        true
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        let Some(target) = &self.target else {
            return false;
        };

        if !target.get_entity().is_alive() {
            return false;
        }

        let navigator = mob
            .get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if navigator.is_idle() {
            return false;
        }

        let mob_pos = mob.get_entity().pos.load();
        let target_pos = target.get_entity().pos.load();
        let dist_sq = mob_pos.squared_distance_to_vec(&target_pos);
        let within_sq = f64::from(self.within) * f64::from(self.within);

        dist_sq < within_sq
    }

    fn start(&mut self, mob: &dyn Mob) {
        if let Some(wanted_pos) = self.wanted_pos {
            let mob_pos = mob.get_entity().pos.load();
            let mut navigator = mob
                .get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            navigator.set_progress(NavigatorGoal::new(mob_pos, wanted_pos, self.speed));
        }
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        self.target = None;
        self.wanted_pos = None;
    }

    fn controls(&self) -> Controls {
        self.goal_control
    }
}

#[cfg(test)]
#[allow(clippy::unimplemented)]
mod tests {
    use super::MoveTowardsTargetGoal;
    use crate::entity::ai::goal::{Controls, Goal};

    #[test]
    fn initial_controls_and_state() {
        let mut goal = MoveTowardsTargetGoal::new(1.0, 16.0);
        assert_eq!(goal.controls(), Controls::MOVE);
        assert!(goal.target.is_none());
        assert!(goal.wanted_pos.is_none());

        goal.wanted_pos = Some(pumpkin_util::math::vector3::Vector3::new(1.0, 2.0, 3.0));
        assert!(goal.wanted_pos.is_some());
        goal.stop(&MockMob);
        assert!(goal.target.is_none());
        assert!(goal.wanted_pos.is_none());
    }

    struct MockMob;
    impl crate::entity::mob::Mob for MockMob {
        fn get_mob_entity(&self) -> &crate::entity::mob::MobEntity {
            unimplemented!()
        }
        fn mob_write_nbt(&self, _nbt: &mut pumpkin_nbt::compound::NbtCompound) {
            unimplemented!()
        }
        fn mob_read_nbt(&self, _nbt: &pumpkin_nbt::compound::NbtCompound) {
            unimplemented!()
        }
    }
}
