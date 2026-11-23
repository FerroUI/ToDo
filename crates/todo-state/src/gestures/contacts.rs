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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContactId {
    Touch(i64),
    Pen(i64),
}

#[derive(Default)]
pub struct Contacts {
    active: Vec<ContactId>,
    suppressed: Vec<ContactId>,
    quarantined: bool,
}

impl Contacts {
    pub fn has_contacts(&self) -> bool {
        !self.active.is_empty()
    }

    pub fn has_pen(&self, id: i64) -> bool {
        self.active.contains(&ContactId::Pen(id))
    }

    /// A host may drain these with native cancellation only after a trusted new
    /// primary Down proves that the interrupted physical sequence has ended.
    pub fn interrupted_ids(&self) -> Vec<i64> {
        if self.quarantined {
            self.active
                .iter()
                .filter_map(|contact| match contact {
                    ContactId::Touch(id) => Some(*id),
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// A cancelled operation stays cancelled until its physical contacts end.
    /// Their End events still reach native devices to dispose delivered pointers.
    pub fn quarantine(&mut self) {
        self.quarantined |= self.has_contacts();
    }

    /// A touch beginning during an operation owned by another input device must
    /// neither become a button press nor resurrect that operation on release.
    pub fn block_begin(&mut self, id: i64) -> ContactDecision {
        self.quarantined = true;
        self.update(id, ContactEvent::Begin, false)
    }

    pub fn block_pen_begin(&mut self, id: i64) -> ContactDecision {
        self.quarantined = true;
        self.update_pen(id, ContactEvent::Begin, false)
    }

    pub fn update_pen(
        &mut self,
        id: i64,
        event: ContactEvent,
        gesture_active: bool,
    ) -> ContactDecision {
        self.update_contact(ContactId::Pen(id), event, gesture_active)
    }

    pub fn update(
        &mut self,
        id: i64,
        event: ContactEvent,
        gesture_active: bool,
    ) -> ContactDecision {
        self.update_contact(ContactId::Touch(id), event, gesture_active)
    }

    fn update_contact(
        &mut self,
        id: ContactId,
        event: ContactEvent,
        gesture_active: bool,
    ) -> ContactDecision {
        match event {
            ContactEvent::Begin => {
                if self.active.contains(&id) {
                    return ContactDecision {
                        cancel: false,
                        suppress: self.quarantined,
                    };
                }
                let mixed = self.active.iter().any(|contact| {
                    matches!(
                        (contact, id),
                        (ContactId::Pen(_), ContactId::Touch(_))
                            | (ContactId::Touch(_), ContactId::Pen(_))
                    )
                });
                self.active.push(id);
                let cancel =
                    !self.quarantined && (gesture_active || mixed) && self.active.len() > 1;
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
                // Native TouchDevice can allocate a pointer on an unknown Move.
                // Only a real accepted Begin may start a new contact sequence.
                suppress: self.quarantined || !self.active.contains(&id),
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
    fn pen_and_touch_have_separate_id_namespaces_and_cancel_each_other() {
        let mut contacts = Contacts::default();
        assert!(!contacts.update_pen(7, ContactEvent::Begin, false).suppress);
        // A second device must cancel even when the original contact belongs
        // to an ordinary native button rather than a task gesture recognizer.
        assert_eq!(
            contacts.update(7, ContactEvent::Begin, false),
            ContactDecision {
                cancel: true,
                suppress: true
            }
        );
        assert!(contacts.update_pen(7, ContactEvent::Move, false).suppress);
        assert!(!contacts.update_pen(7, ContactEvent::End, false).suppress);
        assert!(contacts.has_contacts());
        assert!(contacts.update(7, ContactEvent::Move, false).suppress);
        assert!(contacts.update(7, ContactEvent::End, false).suppress);
        assert!(!contacts.has_contacts());
        assert_eq!(
            contacts.update_pen(8, ContactEvent::Begin, false),
            ContactDecision::default()
        );
    }

    #[test]
    fn browser_touch_recovery_never_relabels_a_pen_as_a_touch() {
        let mut contacts = Contacts::default();
        contacts.update(1, ContactEvent::Begin, false);
        contacts.update_pen(2, ContactEvent::Begin, true);
        assert_eq!(contacts.interrupted_ids(), vec![1]);
        assert!(contacts.has_pen(2));
        assert!(contacts.update_pen(2, ContactEvent::End, false).suppress);
        assert!(!contacts.update(1, ContactEvent::End, false).suppress);
        assert!(!contacts.has_contacts());
    }

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

    #[test]
    fn explicit_interruption_blocks_movement_until_every_contact_is_released() {
        let mut contacts = Contacts::default();
        contacts.update(1, ContactEvent::Begin, false);
        assert!(contacts.interrupted_ids().is_empty());
        contacts.quarantine();
        assert!(contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(contacts.update(1, ContactEvent::Begin, false).suppress);
        assert!(contacts.update(2, ContactEvent::Begin, false).suppress);
        assert_eq!(contacts.interrupted_ids(), vec![1, 2]);
        assert!(!contacts.update(1, ContactEvent::End, false).suppress);
        assert!(contacts.update(2, ContactEvent::Move, false).suppress);
        assert!(contacts.update(2, ContactEvent::End, false).suppress);
        assert!(contacts.interrupted_ids().is_empty());
        assert!(!contacts.has_contacts());
        assert!(!contacts.update(3, ContactEvent::Begin, false).suppress);
    }

    #[test]
    fn a_contact_blocked_by_another_input_device_never_enters_the_native_device() {
        let mut contacts = Contacts::default();
        assert!(contacts.block_begin(1).suppress);
        assert!(contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(contacts.update(1, ContactEvent::End, false).suppress);
        contacts.quarantine();
        assert!(!contacts.update(2, ContactEvent::Begin, false).suppress);
    }

    #[test]
    fn late_or_unknown_movement_cannot_allocate_an_unpressed_native_pointer() {
        let mut contacts = Contacts::default();
        assert!(contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(!contacts.update(1, ContactEvent::Begin, false).suppress);
        assert!(!contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(!contacts.update(1, ContactEvent::End, false).suppress);
        assert!(contacts.update(1, ContactEvent::Move, false).suppress);
        assert!(!contacts.update(2, ContactEvent::Begin, false).suppress);
    }
}
