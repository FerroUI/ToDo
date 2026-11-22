use super::DELETE_REVEAL;

/// Swipe travel and release thresholds follow the actions actually available on a row.
#[derive(Clone, Copy, Debug, Default)]
pub struct SwipeLimits {
    right: f64,
    left: f64,
}

impl SwipeLimits {
    pub fn new(my_day: bool, move_task: bool, delete_task: bool) -> Self {
        Self {
            right: (u8::from(my_day) + u8::from(move_task)) as f64 * DELETE_REVEAL,
            left: f64::from(u8::from(delete_task)) * DELETE_REVEAL,
        }
    }

    pub fn clamp(self, offset: f64) -> f64 {
        if offset.is_finite() {
            offset.clamp(-self.left, self.right)
        } else {
            0.0
        }
    }

    pub fn offset(self, initial: f64, delta: f64) -> f64 {
        self.clamp(initial + delta)
    }

    pub fn settle(self, offset: f64) -> f64 {
        let offset = self.clamp(offset);
        if self.left > 0.0 && offset <= -self.left * 0.4 {
            -self.left
        } else if self.right > 0.0 && offset >= self.right * 0.3 {
            self.right
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gestures::{settle_swipe, swipe_offset, ACTIONS_REVEAL};

    #[test]
    fn ordinary_rows_keep_existing_travel_and_release_behavior() {
        let limits = SwipeLimits::new(true, true, true);
        for initial in [-DELETE_REVEAL, 0.0, ACTIONS_REVEAL] {
            for delta in [-999.0, -60.0, -24.0, 0.0, 24.0, 60.0, 999.0] {
                let offset = limits.offset(initial, delta);
                assert_eq!(offset, swipe_offset(initial, delta));
                assert_eq!(limits.settle(offset), settle_swipe(offset));
            }
        }
    }

    #[test]
    fn one_action_rows_settle_at_a_proportional_threshold_and_close_normally() {
        let planner = SwipeLimits::new(true, false, false);
        assert_eq!(planner.offset(0.0, 999.0), DELETE_REVEAL);
        assert_eq!(planner.settle(24.0), 0.0);
        assert_eq!(planner.settle(30.0), DELETE_REVEAL);
        assert_eq!(planner.settle(planner.offset(DELETE_REVEAL, -80.0)), 0.0);
        assert_eq!(planner.offset(0.0, -999.0), 0.0);
        assert_eq!(planner.settle(-999.0), 0.0);
        let flagged = SwipeLimits::new(true, false, true);
        assert_eq!(flagged.settle(30.0), DELETE_REVEAL);
        assert_eq!(flagged.settle(-40.0), -DELETE_REVEAL);
    }

    #[test]
    fn disappearing_actions_and_invalid_coordinates_cannot_reveal_empty_space() {
        let planner = SwipeLimits::new(true, false, false);
        assert_eq!(planner.clamp(ACTIONS_REVEAL), DELETE_REVEAL);
        assert_eq!(planner.clamp(-DELETE_REVEAL), 0.0);
        for offset in [-999.0, 999.0, f64::NAN, f64::INFINITY] {
            assert_eq!(SwipeLimits::default().settle(offset), 0.0);
        }
    }
}
