use super::{Controls, Goal, to_goal_ticks};
use crate::entity::{ai::pathfinder::NavigatorGoal, mob::Mob};
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

pub const DEFAULT_INTERVAL: i32 = 120;

pub struct WanderAroundGoal {
    goal_control: Controls,
    speed: f64,
    target: Option<Vector3<f64>>,
    interval: i32,
    force_trigger: bool,
}

impl WanderAroundGoal {
    #[must_use]
    pub const fn new(speed: f64) -> Self {
        Self::with_interval(speed, DEFAULT_INTERVAL)
    }

    #[must_use]
    pub const fn with_interval(speed: f64, interval: i32) -> Self {
        Self {
            goal_control: Controls::MOVE,
            speed,
            target: None,
            interval,
            force_trigger: false,
        }
    }

    /// Run once on the next check, ignoring the interval.
    pub const fn trigger(&mut self) {
        self.force_trigger = true;
    }

    pub const fn set_interval(&mut self, interval: i32) {
        self.interval = interval;
    }

    // TODO: replace with a port of `DefaultRandomPos.getPos`, which validates the
    // candidate against the navigation node types instead of picking any offset.
    fn find_wander_target(mob: &dyn Mob) -> Vector3<f64> {
        let entity = &mob.get_mob_entity().living_entity.entity;
        let pos = entity.pos.load();
        let mut rng = mob.get_random();

        let horizontal_range = 10.0;
        let vertical_range = 7.0;

        let dx = rng.random_range(-horizontal_range..=horizontal_range);
        let dy = rng.random_range(-vertical_range..=vertical_range);
        let dz = rng.random_range(-horizontal_range..=horizontal_range);

        Vector3::new(pos.x + dx, pos.y + dy, pos.z + dz)
    }
}

impl Goal for WanderAroundGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        // Every mob that carries a passenger is controlled by it.
        if mob.get_entity().has_passengers() {
            return false;
        }

        if !self.force_trigger {
            // TODO: also bail out on a long no-action time, which needs the despawn bookkeeping.
            if mob
                .get_random()
                .random_range(0..to_goal_ticks(self.interval))
                != 0
            {
                return false;
            }
        }

        self.target = Some(Self::find_wander_target(mob));
        self.force_trigger = false;
        true
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        let idle = mob
            .get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_idle();
        !idle && !mob.get_entity().has_passengers()
    }

    fn start(&mut self, mob: &dyn Mob) {
        if let Some(target) = self.target {
            let pos = mob.get_mob_entity().living_entity.entity.pos.load();
            mob.get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_progress(NavigatorGoal::new(pos, target, self.speed));
        }
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.target = None;
        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
    }

    fn controls(&self) -> Controls {
        self.goal_control
    }
}
