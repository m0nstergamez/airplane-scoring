//! Draws the level background. Where it is comes from core
//! (`background_x`); this file only loads the image and draws copies of it.
//!
//! The image loads in the background like the sounds, so the game starts
//! right away on a plain sky and the picture appears when it is there.

use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;

/// Rendered by `scripts/make-backgrounds.sh` from `art/levels/level1/`.
const LEVEL1: &str = "levels/level1.png";

pub struct Scenery {
    loading: Option<Coroutine<Option<Texture2D>>>,
    texture: Option<Texture2D>,
}

impl Scenery {
    /// Start loading the background image.
    pub fn start() -> Self {
        Self {
            loading: Some(start_coroutine(async {
                load_texture(LEVEL1)
                    .await
                    .map_err(|e| {
                        // Block body: macroquad's `error!` ends in `;`.
                        macroquad::logging::error!("no background: {}: {}", LEVEL1, e);
                    })
                    .ok()
            })),
            texture: None,
        }
    }

    /// Width of one copy of the image on screen: it is scaled to the
    /// screen height. 0 while the image is not loaded.
    pub fn tile_width(&mut self) -> f32 {
        if self.loading.as_ref().is_some_and(Coroutine::is_done) {
            self.texture = self.loading.take().and_then(|l| l.retrieve()).flatten();
        }
        match &self.texture {
            Some(t) => screen_height() * t.width() / t.height(),
            None => 0.0,
        }
    }

    /// Draw the image from `x` (core's `background_x`) to the right until
    /// the screen is covered. Returns false if there is nothing to draw.
    pub fn draw(&self, x: f32) -> bool {
        let Some(texture) = &self.texture else {
            return false;
        };
        let (w, h) = (screen_width(), screen_height());
        let tile = h * texture.width() / texture.height();
        if !tile.is_finite() || tile <= 0.0 {
            return false; // empty image or a 0 px window: would never cover the screen
        }
        let mut left = x;
        while left < w {
            draw_texture_ex(
                texture,
                left,
                0.0,
                WHITE,
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
