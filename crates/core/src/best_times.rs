//! Best times: the fastest completed levels on this device, one table per
//! level. Only a level flown to its landing counts ("Level complete"); a
//! game over or leaving for the menu leaves the table alone. Every level
//! is the same distance ([`crate::LEVEL_DISTANCE`]), so more throttle
//! gets there sooner. Core keeps the rules (what counts, the order, what
//! is a new record); the game crate only loads and saves the numbers (the
//! web page keeps them in the browser's local storage, so they stay on
//! this phone or computer and never go online). Times are milliseconds
//! ([`crate::World::millis_in_level`]), shown as "m:ss.mmm"
//! ([`crate::best_time_text`]).

/// How many best times each level keeps.
pub const BEST_TIMES_COUNT: usize = 5;

/// One level's best times.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BestTimes {
    /// Fastest first, in milliseconds, at most [`BEST_TIMES_COUNT`].
    list: Vec<u64>,
}

/// Where a completed level landed in its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Its place, 0 = the best.
    pub place: usize,
    /// Faster than every earlier time in this level: a new best time.
    pub record: bool,
}

impl BestTimes {
    /// From stored milliseconds, in any order. Zeros are no times; extra
    /// ones beyond the best [`BEST_TIMES_COUNT`] are dropped.
    pub fn from_millis(millis: impl IntoIterator<Item = u64>) -> Self {
        let mut list: Vec<u64> = millis.into_iter().filter(|&s| s > 0).collect();
        list.sort_unstable();
        list.truncate(BEST_TIMES_COUNT);
        Self { list }
    }

    /// The best times in milliseconds, fastest first.
    pub fn list(&self) -> &[u64] {
        &self.list
    }

    /// Add a completed level's time. Returns where it landed, or `None`
    /// if it did not make the table (or is 0). Faster than every earlier
    /// time is a new record; a tie goes below the earlier time, so a
    /// record must beat it.
    pub fn add(&mut self, millis: u64) -> Option<Placed> {
        if millis == 0 {
            return None;
        }
        let place = (self.list.iter())
            .position(|&s| millis < s)
            .unwrap_or(self.list.len());
        if place >= BEST_TIMES_COUNT {
            return None;
        }
        self.list.insert(place, millis);
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
    fn the_first_completed_level_is_a_record() {
        let mut t = BestTimes::default();
        assert_eq!(t.add(0), None, "no time: no entry");
        assert_eq!(t.add(61_250), placed(0, true));
        assert_eq!(t.list(), [61_250]);
    }

    #[test]
    fn times_are_kept_fastest_first_and_only_the_top_five() {
        let mut t = BestTimes::default();
        for ms in [64_000, 61_000, 70_000, 62_500, 65_000, 63_000] {
            t.add(ms);
        }
        assert_eq!(t.list(), [61_000, 62_500, 63_000, 64_000, 65_000]);
        assert_eq!(t.add(80_000), None, "too slow for the table");
        assert_eq!(t.add(62_900), placed(2, false), "in the table, no record");
        assert_eq!(t.list(), [61_000, 62_500, 62_900, 63_000, 64_000]);
    }

    #[test]
    fn only_beating_the_best_is_a_new_record() {
        let mut t = BestTimes::from_millis([60_000]);
        assert_eq!(t.add(60_000), placed(1, false), "a tie is no record");
        assert_eq!(t.add(59_983), placed(0, true), "one tick faster");
        assert_eq!(t.list(), [59_983, 60_000, 60_000]);
    }

    #[test]
    fn stored_times_are_tidied_up() {
        let t = BestTimes::from_millis([3, 0, 90, 12, 7, 55, 1, 0]);
        assert_eq!(t.list(), [1, 3, 7, 12, 55]);
    }
}
