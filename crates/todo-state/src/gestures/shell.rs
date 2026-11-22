//! Signed progress keeps navigation and refresh release decisions on their owned axis.
use super::{Axis, TouchTrack, BACK_DISTANCE, REFRESH_DISTANCE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellGesture {
    Back,
    Drawer,
    Refresh,
}

impl ShellGesture {
    pub fn progress(self, track: &TouchTrack) -> f64 {
        match (self, track.axis()) {
            (Self::Back, Axis::Horizontal) => track.x.max(0.0),
            (Self::Drawer, Axis::Horizontal) => (-track.x).max(0.0),
            (Self::Refresh, Axis::Vertical) => track.y.max(0.0),
            _ => 0.0,
        }
    }

    pub fn ready(self, track: &TouchTrack) -> bool {
        self.progress(track)
            >= if self == Self::Refresh {
                REFRESH_DISTANCE
            } else {
                BACK_DISTANCE
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reversed_navigation_drag_disarms_instead_of_using_absolute_distance() {
        for (gesture, direction) in [(ShellGesture::Back, 1.0), (ShellGesture::Drawer, -1.0)] {
            let mut track = TouchTrack::default();
            track.update(direction * 100.0, 0.0);
            assert!(gesture.ready(&track));
            track.update(-direction * 100.0, 0.0);
            assert_eq!(gesture.progress(&track), 0.0);
            assert!(!gesture.ready(&track));
            track.update(direction * 100.0, 0.0);
            assert!(gesture.ready(&track));
        }
    }

    #[test]
    fn a_pull_can_disarm_but_cannot_become_navigation() {
        let mut track = TouchTrack::default();
        track.update(0.0, 100.0);
        assert!(ShellGesture::Refresh.ready(&track));
        track.update(180.0, 20.0);
        assert!(!ShellGesture::Refresh.ready(&track));
        assert!(!ShellGesture::Back.ready(&track));
        track.update(0.0, -100.0);
        assert_eq!(ShellGesture::Refresh.progress(&track), 0.0);
    }

    #[test]
    fn cancelled_or_invalid_release_cannot_commit_previously_armed_progress() {
        for (gesture, x, y) in [
            (ShellGesture::Back, 100.0, 0.0),
            (ShellGesture::Drawer, -100.0, 0.0),
            (ShellGesture::Refresh, 0.0, 100.0),
        ] {
            let mut track = TouchTrack::default();
            track.update(x, y);
            assert!(gesture.ready(&track));
            track.update(f64::NAN, y);
            assert!(!gesture.ready(&track));
            track.update(x, y);
            assert!(!gesture.ready(&track));
        }
    }
}
