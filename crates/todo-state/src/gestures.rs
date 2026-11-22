//! Device-independent gesture arbitration. Pointer ownership stays in FerroUI's input system.

pub const TOUCH_SLOP: f64 = 8.0;
pub const EDGE_WIDTH: f64 = 24.0;
pub const BACK_DISTANCE: f64 = 72.0;
pub const REFRESH_DISTANCE: f64 = 80.0;
pub const DELETE_REVEAL: f64 = 88.0;
pub const ACTIONS_REVEAL: f64 = 176.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Pending,
    Horizontal,
    Vertical,
    Held,
    Canceled,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TouchTrack {
    axis: Axis,
    pub x: f64,
    pub y: f64,
}

impl TouchTrack {
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// A diagonal movement belongs to scrolling. Once locked, the other axis cannot steal it.
    pub fn update(&mut self, x: f64, y: f64) -> Axis {
        if !x.is_finite() || !y.is_finite() {
            self.cancel();
            return self.axis;
        }
        self.x = x;
        self.y = y;
        if self.axis == Axis::Pending && x.abs().max(y.abs()) >= TOUCH_SLOP {
            self.axis = if x.abs() > y.abs() * 1.35 {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
        }
        self.axis
    }

    /// Native Holding may only claim a stationary contact which scrolling has not already won.
    pub fn hold(&mut self) -> bool {
        if self.axis != Axis::Pending {
            return false;
        }
        self.axis = Axis::Held;
        true
    }

    pub fn moved_after_hold(&self) -> bool {
        self.axis == Axis::Held && self.x.hypot(self.y) >= TOUCH_SLOP
    }

    pub fn cancel(&mut self) {
        self.axis = Axis::Canceled;
    }
}

/// A swipe only reveals buttons. Neither direction commits a task operation by itself.
pub fn swipe_offset(initial: f64, delta: f64) -> f64 {
    (initial + delta).clamp(-DELETE_REVEAL, ACTIONS_REVEAL)
}

pub fn settle_swipe(offset: f64) -> f64 {
    if offset <= -DELETE_REVEAL * 0.4 {
        -DELETE_REVEAL
    } else if offset >= ACTIONS_REVEAL * 0.3 {
        ACTIONS_REVEAL
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_and_diagonal_scrolling_cannot_turn_into_task_actions() {
        for (x, y) in [(1.0, 10.0), (10.0, 10.0), (4.0, -12.0)] {
            let mut track = TouchTrack::default();
            assert_eq!(track.update(x, y), Axis::Vertical);
            assert_eq!(track.update(120.0, y), Axis::Vertical);
            assert!(!track.hold());
        }
    }

    #[test]
    fn a_small_finger_drift_remains_a_tap_or_hold() {
        let mut track = TouchTrack::default();
        assert_eq!(track.update(3.0, -2.0), Axis::Pending);
        assert!(track.hold());
        assert!(!track.moved_after_hold());
        track.update(3.0, 24.0);
        assert!(track.moved_after_hold());
        assert_eq!(track.axis(), Axis::Held);
    }

    #[test]
    fn horizontal_lock_is_stable_and_never_completes_or_deletes_a_task() {
        let mut track = TouchTrack::default();
        assert_eq!(track.update(-18.0, 2.0), Axis::Horizontal);
        assert_eq!(track.update(-80.0, 40.0), Axis::Horizontal);
        assert_eq!(settle_swipe(swipe_offset(0.0, track.x)), -DELETE_REVEAL);
        assert_eq!(settle_swipe(swipe_offset(0.0, 70.0)), ACTIONS_REVEAL);
    }

    #[test]
    fn partial_swipes_close_and_revealed_actions_can_be_swiped_shut() {
        assert_eq!(settle_swipe(24.0), 0.0);
        assert_eq!(settle_swipe(-24.0), 0.0);
        assert_eq!(settle_swipe(swipe_offset(ACTIONS_REVEAL, -160.0)), 0.0);
        assert_eq!(settle_swipe(swipe_offset(-DELETE_REVEAL, 80.0)), 0.0);
        assert_eq!(swipe_offset(0.0, 999.0), ACTIONS_REVEAL);
    }

    #[test]
    fn cancellation_and_invalid_coordinates_cannot_rearm_a_gesture() {
        let mut track = TouchTrack::default();
        track.cancel();
        assert_eq!(track.update(100.0, 0.0), Axis::Canceled);
        assert!(!track.hold());
        let mut track = TouchTrack::default();
        assert_eq!(track.update(f64::NAN, 0.0), Axis::Canceled);
        assert_eq!(track.update(100.0, 0.0), Axis::Canceled);
    }
}
