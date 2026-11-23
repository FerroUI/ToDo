use ferroui_view::prelude::*;
use todo_domain::{RelativeMode, RepeatDay, RepeatUnit, WeekIndex};
use todo_state::RepeatDraft;

pub const UNITS: [RepeatUnit; 5] = [
    RepeatUnit::Day,
    RepeatUnit::Weekday,
    RepeatUnit::Week,
    RepeatUnit::Month,
    RepeatUnit::Year,
];

#[derive(Clone, Copy)]
pub struct Form {
    pub interval: Signal<String>,
    pub unit: Signal<i32>,
    pub start: Signal<String>,
    pub days: Signal<Vec<RepeatDay>>,
    pub week_start: Signal<i32>,
    pub relative: Signal<i32>,
    pub ordinal: Signal<i32>,
    pub relative_mode: Signal<i32>,
    pub day: Signal<String>,
    pub month: Signal<i32>,
    pub ending: Signal<i32>,
    pub until: Signal<String>,
    pub count: Signal<String>,
    pub time_zone: Signal<String>,
}
impl Form {
    pub fn create(initial: &RepeatDraft) -> Self {
        Self {
            interval: use_signal(|| initial.interval.clone()),
            unit: use_signal(|| {
                UNITS
                    .iter()
                    .position(|unit| *unit == initial.unit)
                    .unwrap_or(0) as i32
            }),
            start: use_signal(|| initial.start.clone()),
            days: use_signal(|| initial.days.clone()),
            week_start: use_signal(|| initial.week_start.number() as i32),
            relative: use_signal(|| i32::from(initial.relative)),
            ordinal: use_signal(|| {
                WeekIndex::ALL
                    .iter()
                    .position(|index| *index == initial.ordinal)
                    .unwrap_or(0) as i32
            }),
            relative_mode: use_signal(|| {
                i32::from(initial.relative_mode == RelativeMode::NthMatchingDay)
            }),
            day: use_signal(|| initial.day.clone()),
            month: use_signal(|| i32::from(initial.month) - 1),
            ending: use_signal(|| i32::from(initial.ending)),
            until: use_signal(|| initial.until.clone()),
            count: use_signal(|| initial.count.clone()),
            time_zone: use_signal(|| initial.time_zone.clone()),
        }
    }
    pub fn draft(self) -> RepeatDraft {
        RepeatDraft {
            unit: UNITS
                .get(self.unit.get().max(0) as usize)
                .copied()
                .unwrap_or(RepeatUnit::Day),
            interval: self.interval.get(),
            start: self.start.get(),
            days: self.days.get(),
            week_start: RepeatDay::ALL
                .get(self.week_start.get().max(0) as usize)
                .copied()
                .unwrap_or_default(),
            relative: self.relative.get() == 1,
            ordinal: WeekIndex::ALL
                .get(self.ordinal.get().max(0) as usize)
                .copied()
                .unwrap_or(WeekIndex::First),
            relative_mode: if self.relative_mode.get() == 1 {
                RelativeMode::NthMatchingDay
            } else {
                RelativeMode::FirstMatch
            },
            day: self.day.get(),
            month: (self.month.get() + 1).try_into().unwrap_or(0),
            ending: self.ending.get().try_into().unwrap_or(255),
            until: self.until.get(),
            count: self.count.get(),
            time_zone: self.time_zone.get(),
        }
    }
}
