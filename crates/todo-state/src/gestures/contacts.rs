//! Quarantine a cancelled multi-contact gesture while preserving disposal of existing pointers.

#[derive(Clone, Copy)]
pub enum ContactEvent {
    Begin,
    Move,
    End,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ContactDecision {
    pub cancel: bool,
    pub suppress: bool,
}

#[derive(Default)]
pub struct Contacts {
    active: Vec<i64>,
    suppressed: Vec<i64>,
    quarantined: bool,
}

impl Contacts {
    pub fn update(
        &mut self,
        id: i64,
        event: ContactEvent,
        gesture_active: bool,
    ) -> ContactDecision {
        match event {
            ContactEvent::Begin => {
                if self.active.contains(&id) {
                    return ContactDecision::default();
                }
                self.active.push(id);
                let cancel = !self.quarantined && gesture_active && self.active.len() > 1;
                self.quarantined |= cancel;
                if self.quarantined {
                    self.suppressed.push(id);
                }
                ContactDecision {
                    cancel,
                    suppress: self.quarantined,
                }
            }
            ContactEvent::Move => ContactDecision {
                cancel: false,
                suppress: self.quarantined,
            },
            ContactEvent::End => {
                // A suppressed Begin never reached the native device. Only those pointers'
                // End/Cancel may be suppressed; pre-existing pointers must still be disposed.
                let suppress = self.suppressed.contains(&id);
                self.active.retain(|contact| *contact != id);
                self.suppressed.retain(|contact| *contact != id);
                if self.active.is_empty() {
                    self.quarantined = false;
                }
                ContactDecision {
                    cancel: false,
                    suppress,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_contact_cancels_actions_without_leaking_the_original_pointer() {
        let mut contacts = Contacts::default();
        assert_eq!(
            contacts.update(1, ContactEvent::Begin, false),
            ContactDecision::default()
        );
        assert_eq!(
            contacts.update(2, ContactEvent::Begin, true),
            ContactDecision {
                cancel: true,
                suppress: true
            }
        );
        assert!(contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(contacts.update(2, ContactEvent::Move, false).suppress);
        assert!(contacts.update(2, ContactEvent::End, false).suppress);
        assert!(!contacts.update(1, ContactEvent::End, false).suppress);
        assert_eq!(
            contacts.update(3, ContactEvent::Begin, false),
            ContactDecision::default()
        );
    }

    #[test]
    fn lifting_the_original_contact_does_not_rearm_remaining_fingers() {
        let mut contacts = Contacts::default();
        contacts.update(1, ContactEvent::Begin, false);
        contacts.update(2, ContactEvent::Begin, true);
        assert!(!contacts.update(1, ContactEvent::End, false).suppress);
        assert!(contacts.update(3, ContactEvent::Begin, false).suppress);
        assert!(contacts.update(2, ContactEvent::End, false).suppress);
        assert!(contacts.update(3, ContactEvent::Move, false).suppress);
        assert!(contacts.update(3, ContactEvent::End, false).suppress);
        assert!(!contacts.update(4, ContactEvent::Begin, false).suppress);
    }

    #[test]
    fn native_scroll_contacts_without_an_app_gesture_remain_native() {
        let mut contacts = Contacts::default();
        for id in 1..=2 {
            assert!(!contacts.update(id, ContactEvent::Begin, false).suppress);
        }
        for id in 1..=2 {
            assert!(!contacts.update(id, ContactEvent::End, false).suppress);
        }
    }
}
