//! Calendar scheduling independent of the UI, clock and persistence layer.
//! Days are Gregorian day numbers; weekdays use Monday = 0. Times are local.

#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub enabled: bool,
    pub start_minute: u32,
    pub stop_minute: Option<u32>,
    pub weekdays: [bool; 7],
    pub once_day: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Moment {
    pub day: i32,
    pub weekday: u32,
    pub minute: u32,
}
impl Moment {
    pub fn key(self) -> i64 {
        i64::from(self.day) * 1440 + i64::from(self.minute)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub minute: i64,
    pub action: Action,
}

/// Return the most recent unhandled transition since the last tick. A new process
/// only checks the current minute; it does not start old missed downloads. During
/// a running session, a delayed tick catches up to the final state, avoiding a
/// brief unwanted start when both the start and stop elapsed during sleep.
pub fn due(
    schedule: Schedule,
    now: Moment,
    previous_tick: Option<i64>,
    last_event: Option<i64>,
) -> Option<Event> {
    if !schedule.enabled
        || schedule.start_minute >= 1440
        || schedule.stop_minute.is_some_and(|m| m >= 1440)
        || now.minute >= 1440
        || now.weekday >= 7
    {
        return None;
    }
    let upper = now.key();
    let lower = previous_tick
        .unwrap_or(upper - 1)
        .max(last_event.unwrap_or(i64::MIN));
    if lower >= upper {
        return None;
    }
    let mut latest: Option<Event> = None;
    let mut consider_day = |day: i32| {
        let start = i64::from(day) * 1440 + i64::from(schedule.start_minute);
        let stop = schedule.stop_minute.map(|minute| {
            i64::from(day) * 1440
                + i64::from(minute)
                + if minute < schedule.start_minute {
                    1440
                } else {
                    0
                }
        });
        for event in [
            Some(Event {
                minute: start,
                action: Action::Start,
            }),
            stop.map(|minute| Event {
                minute,
                action: Action::Stop,
            }),
        ]
        .into_iter()
        .flatten()
        {
            // A stop at the same time as a start wins.
            if event.minute > lower
                && event.minute <= upper
                && latest.is_none_or(|old| {
                    event.minute > old.minute
                        || (event.minute == old.minute && event.action == Action::Stop)
                })
            {
                latest = Some(event);
            }
        }
    };
    if let Some(day) = schedule.once_day {
        consider_day(day);
    } else {
        // A weekly schedule's latest transition is at most eight days old,
        // including a stop on the day after its selected start weekday.
        for offset in 0..=7 {
            let weekday = (now.weekday + 7 - offset % 7) % 7;
            if schedule.weekdays[weekday as usize] {
                consider_day(now.day - offset as i32);
            }
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monday_night() -> Schedule {
        Schedule {
            enabled: true,
            start_minute: 23 * 60,
            stop_minute: Some(60),
            weekdays: [true, false, false, false, false, false, false],
            once_day: None,
        }
    }
    fn at(day: i32, weekday: u32, minute: u32) -> Moment {
        Moment {
            day,
            weekday,
            minute,
        }
    }

    #[test]
    fn selected_weekday_starts_but_unselected_day_does_not() {
        assert_eq!(
            due(monday_night(), at(100, 0, 1380), None, None)
                .unwrap()
                .action,
            Action::Start
        );
        assert!(due(monday_night(), at(101, 1, 1380), None, None).is_none());
    }
    #[test]
    fn overnight_stop_belongs_to_previous_selected_day() {
        assert_eq!(
            due(monday_night(), at(101, 1, 60), None, None)
                .unwrap()
                .action,
            Action::Stop
        );
        assert!(due(monday_night(), at(100, 0, 60), None, None).is_none());
    }
    #[test]
    fn sunday_to_monday_wraps() {
        let mut schedule = monday_night();
        schedule.weekdays = [false, false, false, false, false, false, true];
        assert_eq!(
            due(schedule, at(100, 0, 60), None, None).unwrap().action,
            Action::Stop
        );
    }
    #[test]
    fn once_date_ignores_weekday_selection_and_stops_next_day() {
        let mut schedule = monday_night();
        schedule.once_day = Some(101);
        assert_eq!(
            due(schedule, at(101, 1, 1380), None, None).unwrap().action,
            Action::Start
        );
        assert_eq!(
            due(schedule, at(102, 2, 60), None, None).unwrap().action,
            Action::Stop
        );
        assert!(due(schedule, at(108, 1, 1380), None, None).is_none());
    }
    #[test]
    fn restart_and_repeated_tick_do_not_replay_persisted_event() {
        let now = at(100, 0, 1380);
        assert!(due(monday_night(), now, None, Some(now.key())).is_none());
        assert!(due(monday_night(), now, Some(now.key()), None).is_none());
    }
    #[test]
    fn delayed_tick_catches_start_and_final_stop() {
        let before = at(100, 0, 1379).key();
        assert_eq!(
            due(monday_night(), at(100, 0, 1385), Some(before), None)
                .unwrap()
                .action,
            Action::Start
        );
        assert_eq!(
            due(monday_night(), at(101, 1, 65), Some(before), None)
                .unwrap()
                .action,
            Action::Stop
        );
    }
    #[test]
    fn restart_does_not_catch_old_start() {
        assert!(due(monday_night(), at(100, 0, 1385), None, None).is_none());
    }
    #[test]
    fn backward_clock_does_not_replay_events() {
        assert!(
            due(
                monday_night(),
                at(100, 0, 1380),
                Some(at(100, 0, 1390).key()),
                None
            )
            .is_none()
        );
    }
    #[test]
    fn simultaneous_start_and_stop_stays_stopped() {
        let mut schedule = monday_night();
        schedule.stop_minute = Some(schedule.start_minute);
        assert_eq!(
            due(schedule, at(100, 0, 1380), None, None).unwrap().action,
            Action::Stop
        );
    }
    #[test]
    fn disabled_and_invalid_schedules_do_nothing() {
        let mut schedule = monday_night();
        schedule.enabled = false;
        assert!(due(schedule, at(100, 0, 1380), None, None).is_none());
        schedule.enabled = true;
        schedule.start_minute = 1440;
        assert!(due(schedule, at(100, 0, 1380), None, None).is_none());
    }
}
