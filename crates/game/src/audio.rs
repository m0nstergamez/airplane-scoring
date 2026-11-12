//! Plays what core decides. No rules here: core's `Mix` says how loud
//! each loop is, core's `Event`s say when a one-shot sound starts.
//!
//! The files load in the background, so the game starts right away and
//! simply stays silent until they are there (or if they are missing).
//! Browsers only let a page make sound after the first tap, click or key
//! press; `web/index.html` resumes audio on that gesture (the JS glue's
//! own unlock misses iPhones; see docs/README.md).

use airplane_scoring_core::{
    Event, Mix, BALLOON_BURNER_VOLUME, CRASH_VOLUME, EXPLOSION_VOLUME, GEAR_VOLUME,
    GULL_CALL_VOLUME, HIGHSCORE_VOLUME, NEW_LEVEL_VOLUME, TAKEOFF_VOLUME, VOICE_VOLUME,
};
use macroquad::audio::{load_sound, play_sound, set_sound_volume, PlaySoundParams, Sound};
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};

/// Synthesised by `scripts/make-sounds.py` into `web/sounds/`.
struct Sounds {
    engine_idle: Sound,
    engine_power: Sound,
    wind: Sound,
    rain: Sound,
    beach: Sound,
    thunder: Sound,
    crash: Sound,
    new_level: Sound,
    gull_call: Sound,
    burner: Sound,
    fanfare: Sound,
    explosion: Sound,
    overspeed_alarm: Sound,
    stall_alarm: Sound,
    rumble: Sound,
    takeoff: Sound,
    gear: Sound,
    /// The warning voices are optional: without their files (made with
    /// espeak-ng), the game still has every other sound.
    overspeed_voice: Option<Sound>,
    pull_up_voice: Option<Sound>,
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
            beach: file("beach").await?,
            thunder: file("thunder").await?,
            crash: file("crash").await?,
            new_level: file("new-level").await?,
            gull_call: file("gull-call").await?,
            burner: file("balloon-burner").await?,
            fanfare: file("new-highscore").await?,
            explosion: file("explosion").await?,
            overspeed_alarm: file("warning-overspeed").await?,
            stall_alarm: file("warning-stall").await?,
            rumble: file("runway-rumble").await?,
            takeoff: file("takeoff").await?,
            gear: file("gear-up").await?,
            overspeed_voice: file("voice-overspeed").await,
            pull_up_voice: file("voice-pull-up").await,
        })
    }

    fn loops(&self, mix: Mix) -> [(&Sound, f32); 8] {
        [
            (&self.engine_idle, mix.engine_idle),
            (&self.engine_power, mix.engine_power),
            (&self.wind, mix.wind),
            (&self.rain, mix.rain),
            (&self.beach, mix.beach),
            (&self.overspeed_alarm, mix.overspeed),
            (&self.stall_alarm, mix.stall),
            (&self.rumble, mix.rumble),
        ]
    }
}

pub struct Audio {
    loading: Option<Coroutine<Option<Sounds>>>,
    sounds: Option<Sounds>,
    muted: bool,
    /// The "Music" switch: the tunes (new level, highscore fanfare).
    music: bool,
}

impl Audio {
    /// Start loading the sound files.
    pub fn start() -> Self {
        Self {
            loading: Some(start_coroutine(Sounds::load())),
            sounds: None,
            muted: false,
            music: true,
        }
    }

    /// Call once per frame with the current volumes.
    /// `muted`: the "Sounds" switch is off; `music`: the "Music"
    /// switch is on.
    pub fn update(&mut self, mix: Mix, muted: bool, music: bool) {
        self.music = music;
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
            // Cut off a one-shot sound that is still ringing.
            set_sound_volume(&s.thunder, 0.0);
            set_sound_volume(&s.crash, 0.0);
            set_sound_volume(&s.new_level, 0.0);
            set_sound_volume(&s.gull_call, 0.0);
            set_sound_volume(&s.burner, 0.0);
            set_sound_volume(&s.fanfare, 0.0);
            set_sound_volume(&s.explosion, 0.0);
            set_sound_volume(&s.takeoff, 0.0);
            set_sound_volume(&s.gear, 0.0);
            for voice in [&s.overspeed_voice, &s.pull_up_voice].into_iter().flatten() {
                set_sound_volume(voice, 0.0);
            }
        }
        self.muted = muted;
    }

    /// Call for every event of every tick.
    pub fn play(&self, event: Event) {
        let Some(s) = &self.sounds else { return };
        // The tunes follow the "Music" switch, everything else "Sounds".
        let tune = matches!(event, Event::LevelComplete { .. } | Event::NewHighscore);
        if (tune && !self.music) || (!tune && self.muted) {
            return;
        }
        let (sound, volume) = match event {
            Event::Crash => (&s.crash, CRASH_VOLUME),
            Event::Thunder { volume } => (&s.thunder, volume),
            // The tune celebrates the landing; the next level just begins.
            Event::LevelComplete { .. } => (&s.new_level, NEW_LEVEL_VOLUME),
            Event::NewLevel { .. } => return,
            Event::GullCall => (&s.gull_call, GULL_CALL_VOLUME),
            Event::BalloonBurner => (&s.burner, BALLOON_BURNER_VOLUME),
            Event::Lightning { .. } => return, // seen, not heard
            Event::NewHighscore => (&s.fanfare, HIGHSCORE_VOLUME),
            Event::Explosion => (&s.explosion, EXPLOSION_VOLUME),
            Event::Takeoff => (&s.takeoff, TAKEOFF_VOLUME),
            Event::GearUp | Event::GearDown => (&s.gear, GEAR_VOLUME),
            Event::Overspeed => match &s.overspeed_voice {
                Some(voice) => (voice, VOICE_VOLUME),
                None => return,
            },
            Event::PullUp => match &s.pull_up_voice {
                Some(voice) => (voice, VOICE_VOLUME),
                None => return,
            },
            Event::GameOver => return, // the crash was the sound
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
