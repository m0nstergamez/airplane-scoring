//! Plays what core decides. No rules here: core's `Mix` says how loud
//! each loop is, core's `Event`s say when a one-shot sound starts.
//!
//! The files load in the background, so the game starts right away and
//! simply stays silent until they are there (or if they are missing).
//! Browsers only let a page make sound after the first tap, click or key
//! press; the JS glue (`mq_js_bundle.js`) resumes audio on that gesture.

use airplane_scoring_core::{Event, Mix, CRASH_VOLUME};
use macroquad::audio::{load_sound, play_sound, set_sound_volume, PlaySoundParams, Sound};
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};

/// Synthesised by `scripts/make-sounds.py` into `web/sounds/`.
struct Sounds {
    engine_idle: Sound,
    engine_power: Sound,
    wind: Sound,
    rain: Sound,
    thunder: Sound,
    crash: Sound,
}

impl Sounds {
    async fn load() -> Option<Sounds> {
        async fn file(name: &str) -> Option<Sound> {
            let path = format!("sounds/{name}.wav");
            load_sound(&path)
                .await
                .map_err(|e| {
                    // Block body: macroquad's `error!` ends in `;`, which newer
                    // rustc rejects in expression position.
                    macroquad::logging::error!("no sound: {}: {}", path, e);
                })
                .ok()
        }
        Some(Sounds {
            engine_idle: file("engine-idle").await?,
            engine_power: file("engine-power").await?,
            wind: file("wind").await?,
            rain: file("rain").await?,
            thunder: file("thunder").await?,
            crash: file("crash").await?,
        })
    }

    fn loops(&self, mix: Mix) -> [(&Sound, f32); 4] {
        [
            (&self.engine_idle, mix.engine_idle),
            (&self.engine_power, mix.engine_power),
            (&self.wind, mix.wind),
            (&self.rain, mix.rain),
        ]
    }
}

pub struct Audio {
    loading: Option<Coroutine<Option<Sounds>>>,
    sounds: Option<Sounds>,
    muted: bool,
}

impl Audio {
    /// Start loading the sound files.
    pub fn start() -> Self {
        Self {
            loading: Some(start_coroutine(Sounds::load())),
            sounds: None,
            muted: false,
        }
    }

    /// Call once per frame with the current volumes.
    pub fn update(&mut self, mix: Mix, muted: bool) {
        if self.loading.as_ref().is_some_and(Coroutine::is_done) {
            self.sounds = self.loading.take().and_then(|l| l.retrieve()).flatten();
            if let Some(s) = &self.sounds {
                // The loops play all the time; only their volume changes.
                for (sound, _) in s.loops(Mix::SILENT) {
                    play_sound(
                        sound,
                        PlaySoundParams {
                            looped: true,
                            volume: 0.0,
                        },
                    );
                }
            }
        }
        let Some(s) = &self.sounds else { return };
        let mix = if muted { Mix::SILENT } else { mix };
        for (sound, volume) in s.loops(mix) {
            set_sound_volume(sound, volume);
        }
        if muted && !self.muted {
            // Cut off a thunder or crash that is still ringing.
            set_sound_volume(&s.thunder, 0.0);
            set_sound_volume(&s.crash, 0.0);
        }
        self.muted = muted;
    }

    /// Call for every event of every tick.
    pub fn play(&self, event: Event) {
        let Some(s) = &self.sounds else { return };
        if self.muted {
            return;
        }
        let (sound, volume) = match event {
            Event::Crash => (&s.crash, CRASH_VOLUME),
            Event::Thunder { volume } => (&s.thunder, volume),
            Event::Lightning { .. } => return, // seen, not heard
        };
        play_sound(
            sound,
            PlaySoundParams {
                looped: false,
                volume,
            },
        );
    }
}
