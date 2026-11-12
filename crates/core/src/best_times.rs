//! Best times: the longest flights on this device, one table per level
//! (the level the flight started in, picked on the level map). Core keeps
//! the rules (what counts, the order, what is a new record); the game
//! crate only loads and saves the numbers (the web page keeps them in the
//! browser's local storage, so they stay on this phone or computer and
//! never go online). Times are whole seconds flown
//! ([`crate::World::seconds_flown`]), shown as "m:ss" ([`crate::clock_text`]).

/// How many best times each level keeps.
pub const BEST_TIMES_COUNT: usize = 5;

/// One level's best times.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BestTimes {
    /// Longest first, at most [`BEST_TIMES_COUNT`].
    list: Vec<u64>,
}

/// Where a flight landed in its level's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Its place, 0 = the best.
    pub place: usize,
    /// Longer than every earlier flight in this level: a new best time.
    pub record: bool,
}

impl BestTimes {
    /// From stored seconds, in any order. Zeros are not flights; extra
    /// ones beyond the best [`BEST_TIMES_COUNT`] are dropped.
    pub fn from_seconds(seconds: impl IntoIterator<Item = u64>) -> Self {
        let mut list: Vec<u64> = seconds.into_iter().filter(|&s| s > 0).collect();
        list.sort_unstable_by(|a, b| b.cmp(a));
        list.truncate(BEST_TIMES_COUNT);
        Self { list }
    }

    /// The best times in seconds, longest first.
    pub fn list(&self) -> &[u64] {
        &self.list
    }

    /// Add a finished flight's time. Returns where it landed, or `None`
    /// if it did not make the table (or lasted no second at all). Longer
    /// than every earlier time is a new record; a tie goes below the
    /// earlier time, so a record must beat it.
    pub fn add(&mut self, seconds: u64) -> Option<Placed> {
        if seconds == 0 {
            return None;
        }
        let place = (self.list.iter())
            .position(|&s| seconds > s)
            .unwrap_or(self.list.len());
        if place >= BEST_TIMES_COUNT {
            return None;
        }
        self.list.insert(place, seconds);
        self.list.truncate(BEST_TIMES_COUNT);
        Some(Placed {
            place,
            record: place == 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(place: usize, record: bool) -> Option<Placed> {
        Some(Placed { place, record })
    }

    #[test]
    fn starts_empty() {
        assert!(BestTimes::default().list().is_empty());
    }

    #[test]
    fn the_first_flight_is_a_record_if_it_lasted() {
        let mut t = BestTimes::default();
        assert_eq!(t.add(0), None, "not even a second: no entry");
        assert_eq!(t.add(30), placed(0, true));
        assert_eq!(t.list(), [30]);
    }

    #[test]
    fn times_are_kept_longest_first_and_only_the_top_five() {
        let mut t = BestTimes::default();
        for s in [40, 10, 70, 20, 50, 30] {
            t.add(s);
        }
        assert_eq!(t.list(), [70, 50, 40, 30, 20]);
        assert_eq!(t.add(5), None, "too short for the table");
        assert_eq!(t.add(45), placed(2, false), "in the table, no record");
        assert_eq!(t.list(), [70, 50, 45, 40, 30]);
    }

    #[test]
    fn only_beating_the_best_is_a_new_record() {
        let mut t = BestTimes::from_seconds([100]);
        assert_eq!(t.add(100), placed(1, false), "a tie is no record");
        assert_eq!(t.add(101), placed(0, true));
        assert_eq!(t.list(), [101, 100, 100]);
    }

    #[test]
    fn stored_times_are_tidied_up() {
        let t = BestTimes::from_seconds([3, 0, 90, 12, 7, 55, 1, 0]);
        assert_eq!(t.list(), [90, 55, 12, 7, 3]);
    }
}
