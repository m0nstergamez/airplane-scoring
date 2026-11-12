//! Highscores: the best scores on this device. Core keeps the rules
//! (what counts, the order, what is a new record); the game crate only
//! loads and saves the numbers (the web page keeps them in the browser's
//! local storage, so they stay on this phone or computer and never go
//! online).

use crate::Event;

/// How many best scores are kept.
pub const HIGHSCORE_COUNT: usize = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Highscores {
    /// Best first, at most [`HIGHSCORE_COUNT`].
    list: Vec<u64>,
}

impl Highscores {
    /// From stored numbers, in any order. Zeros are not scores; extra
    /// ones beyond the best [`HIGHSCORE_COUNT`] are dropped.
    pub fn from_scores(scores: impl IntoIterator<Item = u64>) -> Self {
        let mut list: Vec<u64> = scores.into_iter().filter(|&s| s > 0).collect();
        list.sort_unstable_by(|a, b| b.cmp(a));
        list.truncate(HIGHSCORE_COUNT);
        Self { list }
    }

    /// The best scores, best first.
    pub fn list(&self) -> &[u64] {
        &self.list
    }

    /// The best score so far, 0 if none.
    pub fn best(&self) -> u64 {
        self.list.first().copied().unwrap_or(0)
    }

    /// Add a finished flight's score. Returns its place in the list (0 =
    /// best), or `None` if it did not make it. A score better than every
    /// earlier one is a new record and sends [`Event::NewHighscore`]; the
    /// very first flight counts as a record only if it scored anything.
    /// A tie goes below the earlier score, so a record must beat it.
    pub fn add(&mut self, score: u64, events: &mut Vec<Event>) -> Option<usize> {
        if score == 0 {
            return None;
        }
        let place = self
            .list
            .iter()
            .position(|&s| score > s)
            .unwrap_or(self.list.len());
        if place >= HIGHSCORE_COUNT {
            return None;
        }
        if place == 0 {
            events.push(Event::NewHighscore);
        }
        self.list.insert(place, score);
        self.list.truncate(HIGHSCORE_COUNT);
        Some(place)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(h: &mut Highscores, score: u64) -> (Option<usize>, Vec<Event>) {
        let mut events = Vec::new();
        let place = h.add(score, &mut events);
        (place, events)
    }

    #[test]
    fn starts_empty() {
        let h = Highscores::default();
        assert_eq!(h.best(), 0);
        assert!(h.list().is_empty());
    }

    #[test]
    fn the_first_flight_is_a_record_if_it_scored() {
        let mut h = Highscores::default();
        assert_eq!(add(&mut h, 0), (None, vec![]), "no points, no entry");
        assert_eq!(add(&mut h, 30), (Some(0), vec![Event::NewHighscore]));
        assert_eq!(h.best(), 30);
    }

    #[test]
    fn scores_are_kept_best_first_and_only_the_top_five() {
        let mut h = Highscores::default();
        for s in [40, 10, 70, 20, 50, 30] {
            add(&mut h, s);
        }
        assert_eq!(h.list(), [70, 50, 40, 30, 20]);
        assert_eq!(add(&mut h, 5), (None, vec![]), "too low for the list");
        assert_eq!(add(&mut h, 45), (Some(2), vec![]), "in the list, no record");
        assert_eq!(h.list(), [70, 50, 45, 40, 30]);
    }

    #[test]
    fn only_beating_the_best_is_a_new_record() {
        let mut h = Highscores::from_scores([100]);
        assert_eq!(add(&mut h, 100), (Some(1), vec![]), "a tie is no record");
        assert_eq!(add(&mut h, 101), (Some(0), vec![Event::NewHighscore]));
        assert_eq!(h.list(), [101, 100, 100]);
    }

    #[test]
    fn stored_scores_are_tidied_up() {
        let h = Highscores::from_scores([3, 0, 90, 12, 7, 55, 1, 0]);
        assert_eq!(h.list(), [90, 55, 12, 7, 3]);
    }
}
