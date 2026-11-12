//! Draws the level backgrounds. Which level shows, how far the next one
//! has faded in and where the image is all come from core (`level_view`,
//! `background_x`); this file only loads the images and draws copies.
//!
//! The images load in the background like the sounds, all at the start,
//! so the next level is ready when it fades in. Until an image is there,
//! the game shows a plain sky in its place.

use airplane_scoring_core::LEVEL_COUNT;
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;

/// One level's image, rendered by `scripts/make-backgrounds.sh` from
/// `art/levels/levelN/` into `web/levels/levelN.png`.
struct Background {
    loading: Option<Coroutine<Option<Texture2D>>>,
    texture: Option<Texture2D>,
}

impl Background {
    fn start(level: u32) -> Self {
        let path = format!("levels/level{level}.png");
        Self {
            loading: Some(start_coroutine(async move {
                load_texture(&path)
                    .await
                    .map_err(|e| {
                        // Block body: macroquad's `error!` ends in `;`.
                        macroquad::logging::error!("no background: {}: {}", path, e);
                    })
                    .ok()
            })),
            texture: None,
        }
    }

    fn poll(&mut self) {
        if self.loading.as_ref().is_some_and(Coroutine::is_done) {
            self.texture = self.loading.take().and_then(|l| l.retrieve()).flatten();
        }
    }
}

pub struct Scenery {
    levels: Vec<Background>,
}

impl Scenery {
    /// Start loading every level's image.
    pub fn start() -> Self {
        Self {
            levels: (1..=LEVEL_COUNT).map(Background::start).collect(),
        }
    }

    fn texture(&self, level: u32) -> Option<&Texture2D> {
        let i = level.checked_sub(1)? as usize;
        self.levels.get(i)?.texture.as_ref()
    }

    /// Pick up images that finished loading. Call once per frame.
    pub fn update(&mut self) {
        self.levels.iter_mut().for_each(Background::poll);
    }

    /// Width of one copy of `level`'s image on screen: it is scaled to the
    /// screen height. 0 while the image is not loaded.
    pub fn tile_width(&self, level: u32) -> f32 {
        match self.texture(level) {
            Some(t) => screen_height() * t.width() / t.height(),
            None => 0.0,
        }
    }

    /// Draw `level`'s image from `x` (core's `background_x`) to the right
    /// until the screen is covered, `alpha` = how opaque (0 to 1).
    /// Returns false if there is nothing to draw.
    pub fn draw(&self, level: u32, x: f32, alpha: f32) -> bool {
        let Some(texture) = self.texture(level) else {
            return false;
        };
        let (w, h) = (screen_width(), screen_height());
        let tile = self.tile_width(level);
        if !tile.is_finite() || tile <= 0.0 {
            return false; // empty image or a 0 px window: would never cover the screen
        }
        let mut left = x;
        while left < w {
            draw_texture_ex(
                texture,
                left,
                0.0,
                Color::new(1.0, 1.0, 1.0, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(tile, h)),
                    ..Default::default()
                },
            );
            left += tile;
        }
        true
    }
}
