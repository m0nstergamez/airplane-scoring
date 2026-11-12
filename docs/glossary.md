# Glossary / Wörterbuch (de ↔ en)

The agreed translations for the manual (`docs/content/{de,en}`) and the
game's texts. Use exactly these terms so both languages stay consistent.
When you introduce a new term, add it here in the same PR.

Columns: **Deutsch**, **English**, the name **in code** (if any), and a
short **meaning**.

## Game and controls

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Flugzeug | plane | `Player`, `Plane` | The plane the player flies |
| steuern | steer | `steer` | Make the plane climb or dive |
| Steuerwert | steer value | `steer`, `Input` | One number from −1 (climb) to +1 (dive) that every control produces |
| steigen | climb | `steer < 0` | Fly upwards |
| sinken | dive | `steer > 0` | Fly downwards |
| geradeaus fliegen | fly level | `Input::NONE` | Neither climb nor dive |
| kippen | tilt | `Tilt` | Steer by rotating the phone like a steering wheel |
| gerade (Haltung) | level (position) | neutral | How the phone is held at "Tap to fly"; counts as no steering |
| neu kalibrieren | recalibrate | `recalibrate()` | Make the current phone position "level" again |
| Totzone | dead zone | `TILT_DEAD_ZONE_DEG` | Small turns (±2°) that are ignored |
| volle Geschwindigkeit | full speed | `TILT_FULL_DEG` | Steering at full strength (from 25° on) |
| Glättung | smoothing | `TILT_SMOOTHING` | Evens out shaky hands |
| Finger | finger | `pointer` | Steering by touching the screen |
| Taste | key | `steer_from_keys` | Keyboard key (↑/W, ↓/S) |
| Spielerin / Spieler (kurz: Spieler:in) | player | — | Person playing. In running text «Spielerin oder Spieler», in short labels «Spieler:in» |
| Hindernis | obstacle | `Balloons`, `Pad`, `Huts`, `Gulls`, `Towers` | Something you must not fly into; touching it is a crash. So far: hot-air balloons, launch pads and huts in level 1, gulls and lifeguard towers in level 2 |
| Leben | life, lives | `World.lives`, `LIVES` | Crashes you can afford; shown as small planes at the top right |
| Rauchfahne | smoke trail | `World::smoke`, `DAMAGE_SMOKE` | Smoke behind a damaged plane; thicker with every life lost |
| Fluggeschwindigkeit | airspeed | `Player::airspeed`, `Input::throttle` | How fast you fly yourself: gas (→/D, tip forward) and brake (←/A, tip back) |
| Overspeed | overspeed | `Warning::Overspeed`, `OVERSPEED_AT` | Too fast; after a countdown the plane breaks apart. On-screen label and voice stay English |
| Pull up | pull up | `Warning::PullUp`, `STALL_AT` | Too slow; the plane sags. On-screen label and voice stay English |
| Tempo | speed | `World::speed`, `SPEED_PER_POINT` | How fast the landscape comes by; a little faster with every point |
| Wackeln | shake | `World::shake`, `SHAKE_PX` | The screen shaking at the explosion |
| Explosion | explosion | `Event::Explosion`, `explosion.wav` | The plane blowing up when the last life goes |
| geschützt (blinken) | safe (blinking) | `Player::is_safe`, `SAFE_SECONDS` | After a crash the plane blinks and cannot lose another life |
| Game over | game over | `Event::GameOver`, `World::is_over` | The last life is gone; the flight ends. On-screen label, stays English |
| Highscore | highscore | `Highscores`, `Event::NewHighscore` | The best scores on this device (top 5); a new best one is a new highscore. On-screen labels «Best», «Highscores», «New highscore!» stay English |
| Punkte, Score | points, score | `World::score`, `SCORE_PER_SECOND` | 2 per second flown; shown at the top centre and at game over. On-screen label «Score» stays English |

## Weather and sound

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Wetter | weather | `Weather` | Rain, wind and thunderstorms; changes while you fly |
| Regen | rain | `Weather::rain` | From 0 (dry) to 1 (downpour) |
| Wind | wind | `Weather::wind` | From 0 to 1; grows with the rain |
| Böe | gust | `GUST_WIND` | A short burst of extra wind |
| Gewitter | thunderstorm | `STORM_RAIN` | Rain heavy enough for lightning |
| Blitz | lightning | `Event::Lightning` | Lights up the sky |
| Donner | thunder | `Event::Thunder` | Heard after the lightning; later and quieter from far away |
| Zusammenstoss | crash | `Event::Crash`, `CRASH_SECONDS` | The player's plane touches an obstacle, loses a life and is out of control for a moment |
| Ton | sound | `crates/game/src/audio.rs` | Everything you hear. «Ton ein/aus» = sound on/off |
| Tondatei | sound file | `web/sounds/*.wav` | One recorded or computed sound |
| Lautstärke | volume | `Mix` | How loud a sound plays, 0 to 1 |
| Mischung | mix | `Mix` | The volumes of all looping sounds at one moment |
| Schleife | loop | `looped: true` | A sound that repeats without a gap (engine, wind, rain) |
| Triebwerk (Motor) | engine | `engine_idle`, `engine_power` | The plane's jet engine sound: turbine whine and roar; quieter the further away |
| Leerlauf | idle | `engine_idle` | Engine at low power (diving) |
| Vollgas | full power | `engine_power` | Engine at high power (climbing) |
| stummschalten | mute | `sound_js.rs` | Switch all sound off |
| Lautsprecher-Knopf | speaker button | `#mute` | Button at the top right for sound on/off |
| Ereignis | event | `Event` | Something that happened in one tick |
| Wellen | waves | `beach.wav` | Sea sound at the beach (level 2) |
| Heissluftballon | hot-air balloon | `Balloon`, `BalloonWay` | Level 1's obstacle: drifts left while taking off from a launch pad, landing on one, coming at the plane's height, or sinking past; the four ways take turns; envelope and basket both count |
| Startrampe | launch pad | `Pad` | Flat deck in the grass where balloons take off and land; an obstacle |
| Hütte | hut | `Hut`, `Huts` | Big wooden hut in level 1's foreground, at the bottom of the screen; an obstacle |
| Brenner | burner | `Event::BalloonBurner`, `balloon-burner.wav` | The roar of a balloon's gas flame, heard as it appears |
| Strandturm | lifeguard tower | `Tower`, `Towers` | Wooden lifeguard tower in the beach's sand (level 2), near the front or back by the water; an obstacle |
| Parallaxe | parallax | `Tower::size` | Things further away slide past more slowly |
| Möwe | gull | `Gull`, `gull-call.wav` | Sea bird at the beach: an obstacle; many call when they appear |
| Melodie | tune | `Event::NewLevel`, `new-level.wav` | Short rising tune when a new level begins |
| Startwert | seed | `World::with_seed` | The number that decides the random weather |

## Pictures and levels

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Level | level | `art/levels/levelN` | One part of the game with its own landscape. «das Level», plural «die Levels» |
| Massstab | scale | `Bounds::scale`, `DESIGN_HEIGHT` | How much bigger everything is than on a phone (screen height / 390); keeps plane, obstacles and speeds in proportion to the landscape |
| Objektgrösse | object size | `OBJECT_SIZE`, `Bounds::size` | How big the plane and obstacles are drawn and hit, relative to the first design (0.41), up in the sky; not the huts and pads, which belong to the landscape |
| Trefferform | hit shape | `Poly`, `Player::shape` | The drawn outline used for crashes; only a real overlap counts |
| Neigung | pitch | `Player::pitch`, `MAX_PITCH_DEG` | How the plane's nose tilts: up to climb, down to dive |
| Tiefe | depth | `Bounds::depth`, `Bounds::plane_depth` | What flies is smallest at the top (far away) and grows steadily down to the huts; the plane grows the most |
| Hintergrund | background | `crates/game/src/scenery.rs` | The landscape behind the plane |
| Landschaft | landscape | `Scenery` | What the background shows |
| Vorlage (Foto) | photo (to trace) | `art/levels/levelN/photo.jpg` | The photo a background is drawn from; never committed |
| Zeichnung | drawing | `background.svg` | The SVG drawn from the photo; the file you edit |
| scrollen | scroll | `SCROLL_SPEED` | The landscape slides past the plane |
| Kopie | copy | `background_x` | The background drawn once more beside itself |
| Naht | seam | `make-backgrounds.sh` | Where one copy touches the next; must not be visible |
| Levelwechsel | level change | `level_view`, `LEVEL_SECONDS` | Every minute the next level begins; after the last one, level 1 again |
| einblenden | fade in | `LEVEL_FADE_SECONDS`, `LevelView::fade` | The next landscape appears slowly over the old one |
| Ebene | layer | `<g id="sky">` | One group in the SVG (sky, hills, …); can later scroll at its own speed |

## Phone and screen

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Handy | phone | — | Smartphone (iPhone or Android) |
| Querformat | landscape | `orientation: landscape` | Phone held sideways; the game's format |
| Hochformat | portrait | `orientation: portrait` | Phone held upright |
| quer drehen | turn sideways | — | Rotate the phone into landscape |
| Ausrichtungssperre | rotation lock | — | iPhone setting; Android: «Automatisch drehen» / "Auto-rotate" |
| Lautlos-Modus | silent mode | — | iPhone switch on the side; also mutes the game |
| Bewegung & Ausrichtung | Motion & Orientation | `DeviceMotionEvent` | iPhone permission needed for tilting |
| Home-Bildschirm | home screen | `manifest.webmanifest` | iPhone: «Zum Home-Bildschirm» / "Add to Home Screen" |
| Startbildschirm | home screen | `manifest.webmanifest` | Android: «Zum Startbildschirm hinzufügen» / "Add to Home screen" |
| Startanzeige | start screen | `#start` | Shown before flying: "Tap to fly" on phones, "Click to fly" with keys on computers; both with a QR code |
| QR-Code | QR code | `web/qr.svg` | Square code on the start screen; a phone camera opens the game with it |
| Vollbild | full screen | `display: fullscreen` | Game fills the whole screen |
| Bildschirm | screen | — | |

## How the game works

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Regeln | rules | `crates/core` | Everything the game decides; no graphics, all tested |
| Spielschleife | game loop | `main` loop | Read input → ticks → draw, again and again |
| Tick | tick | `World::tick`, `TICK_SECONDS` | One fixed step of 1/60 second |
| Takt | clock | `Clock` | Turns the time between two frames into ticks; gives none before "Tap to fly" |
| Bild | frame | `next_frame()` | One drawn picture on the screen |
| zeichnen | draw | `crates/game` | Put the game on the screen |
| Eingabe | input | `Input` | What the player does (finger, keys, tilt) |
| Welt | world | `World` | Everything that exists in the game right now |

## Development

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Anleitung | manual | `docs/content` | This manual (player + developer part) |
| Projektnotizen | project notes | `docs/README.de.md`, `docs/README.md` | Dated design decisions and open questions, in German and English |
| Entwickeln | developing | — | Changing the game |
| bauen | build | `scripts/build-web.sh` | Turn the code into the playable game |
| Prüfungen | checks | `scripts/check.sh` | Tests, formatting, lints, build, screenshot |
| Test | test | `#[test]` | Code that checks one rule automatically |
| rot / grün | red / green | — | Test fails / passes |
| aufräumen | tidy up | — | Improve code without changing behaviour (refactor) |
| Konstante | constant | `const` | A fixed number you can try changing |
| Webseite | web page | `web/index.html` | The page that loads the game |
| Diagramm | diagram | Mermaid | Picture that explains a concept |

## Not translated

These stay English in German texts too: **Branch, Commit (committen),
Pull Request, Merge (mergen), Tick, WebAssembly, GitHub Pages, CI**, and
on-screen labels exactly as the game shows them (**"Tap to fly"**,
**"Click to fly"**, **"Turn your phone sideways"**).
