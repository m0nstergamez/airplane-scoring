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
| Hindernis | obstacle | `Balloons`, `Pad`, `Huts`, `Gulls`, `Towers`, `City`, `Sunrise` | Something you must not fly into; touching it is a crash. So far: hot-air balloons, launch pads and huts in level 1, gulls and lifeguard towers in level 2, high-rises, cranes and helicopters in level 3, geese, houses and high-rises in level 4 |
| Leben | life, lives | `World.lives`, `LIVES` | Crashes you can afford; shown as small planes at the top right |
| Rauchfahne | smoke trail | `World::smoke`, `DAMAGE_SMOKE` | Smoke behind a damaged plane; thicker with every life lost |
| Fluggeschwindigkeit | airspeed | `Player::airspeed`, `Input::throttle` | How fast you fly yourself: gas (→/D, tip forward) and brake (←/A, tip back) |
| Overspeed | overspeed | `Warning::Overspeed`, `OVERSPEED_AT` | Too fast; after a countdown the plane breaks apart. On-screen label and voice stay English |
| Pull up | pull up | `Warning::PullUp`, `STALL_AT` | Too slow; the plane sags. On-screen label and voice stay English |
| Tempo | speed | `World::speed`, `SPEED_PER_SECOND` | How fast the landscape comes by; a little faster with every second flown |
| Wackeln | shake | `World::shake`, `SHAKE_PX` | The screen shaking at the explosion |
| Explosion | explosion | `Event::Explosion`, `explosion.wav` | The plane blowing up when the last life goes |
| geschützt (blinken) | safe (blinking) | `Player::is_safe`, `SAFE_SECONDS` | After a crash the plane blinks and cannot lose another life |
| Game over | game over | `Event::GameOver`, `World::is_over` | The last life is gone; the flight ends. On-screen label, stays English |
| Bestzeiten | best times | `BestTimes`, `BEST_TIMES_COUNT`, `best_time_text` | The 5 fastest completed levels per level, in milliseconds («1:02.483»); only «Level complete» records one. Shown at «Level complete» and game over. On-screen labels «Best times», «New best time!» stay English |
| Bestenliste | leaderboard | `crates/server`, `board.rs`, `#board-button` | The online list of the fastest completed level times of everybody, one per level; only when a server is set (`board-api`). On-screen label «Leaderboard» stays English |
| Weltrang | world rank | `board::Standing` | Your place on a level's leaderboard, shown at «Level complete» («World rank 3 of 120», stays English) |
| Pilotin / Pilot | pilot | `players`, token | Someone on the leaderboard: anonymous, known to the server only by a secret token kept in the browser |
| Stage | stage | `scripts/stage.sh` | The leaderboard as a whole on the developer's Mac (OrbStack VM, real server image) to test before it goes online. Stays English |
| Replay | replay | `replay.rs`, `Recorder`, `replay()` | A flight written down as its seed (Startwert) and every input, so the server can fly it again and read the time itself |
| Strecke | distance | `LEVEL_DISTANCE`, `World::level_progress` | How far one level is; more throttle covers it sooner |
| Fortschrittsanzeige | progress bar | `World::level_progress` | The thin bar under the clock: a small plane flying to the chequered flag, where the landing begins |
| Flugzeit, Zeit | flight time, time | `World::seconds_in_level`, `World::seconds_flown`, `clock_text` | How long you have flown in this level, in minutes and seconds («1:05»), from 0:00 in every level (`seconds_flown`: the whole flight); shown at the top centre, at «Level complete» and at game over. On-screen label «Time» stays English |

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
| Heissluftballon | hot-air balloon | `Balloon`, `BalloonWay` | Level 1's obstacle: drifts left while taking off from a launch pad, landing on one, coming at the plane's height, or sinking past from it; the four ways take turns; envelope and basket both count |
| Startrampe | launch pad | `Pad` | Flat deck in the grass where balloons take off and land; an obstacle |
| Menü, Pause | menu, pause | `#menu`, `page_started` | The pause menu (button top left, Esc, P): Continue, Restart, Calibrate, Options, Menu. Labels stay English |
| Hauptmenü | main menu | `#start` | The start page: title, plane, Tap to fly, Levels, Options |
| Spielanleitung | how to play | `#start .howto` | The short guide in the main menu: goal, steering, speed, pause. On-screen text stays English |
| Level-Karte | level map | `#start .map` | Round pictures of each level's landscape on a dashed path |
| freischalten | unlock | `airplane-scoring.unlocked` | Landing a level opens the next one on the map |
| Countdown | countdown | `#countdown` | 3, 2, 1 before the game goes on after the pause |
| Musik / Geräusche | music / sounds | `sound_music_on`, `sound_muted` | The two switches in Options: the tunes, and everything else |
| Flughafen | airport | `Airport`, `World::with_takeoff` | Where every flight starts: runway, control tower, windsock |
| Fahrwerk | landing gear | `World::gear`, `Event::GearUp`, `gear-up.wav` | The wheels; they fold up after lift-off with a whir and a clunk |
| Startbahn | runway | `RUNWAY_LENGTH`, `ROLL_SECONDS` | The plane rolls on it and lifts off; nothing else stands on it |
| Hütte | hut | `Hut`, `Huts` | Big wooden hut in level 1's foreground, at the bottom of the screen; an obstacle |
| Brenner | burner | `Event::BalloonBurner`, `balloon-burner.wav` | The roar of a balloon's gas flame, heard as it appears |
| Strandturm | lifeguard tower | `Tower`, `Towers` | Wooden lifeguard tower in the beach's sand (level 2), near the front or back by the water; an obstacle |
| Stadt | town | `City`, `CITY_LEVEL` | Level 3: a town at night, with high-rises and cranes |
| Hochhaus | high-rise | `HighRise` | Tall building at the front of the town (level 3), with an antenna; fly over it; an obstacle |
| Baukran, Kran | tower crane, crane | `Crane` | Crane standing on the ground in the town (level 3), mast, jib and load; fly over it; an obstacle |
| Helikopter | helicopter | `Heli`, `HeliState`, `FlyingHeli` | Level 3: on some high-rises (takes off or lands far ahead; parked, fly over it) and flying at the plane at its height; hit, it explodes |
| Sonnenaufgang | sunrise | `Sunrise`, `SUNRISE_LEVEL` | Level 4: flying toward the low sun over a forest and a hazy valley |
| Gans, V | goose, V | `Goose` | Level 4: geese come at the plane in a V aimed at its height; an obstacle, knocked down when hit |
| Dampf, Dampfsäule | steam, column of steam | `Steam`, `STEAM_LIFT` | Level 4: rises from a house's chimney in the valley; throws the plane up, costs no life |
| Hochhaus im Tal | valley high-rise | `FarHighRise` | Level 4: a high-rise in the valley, black against the sun; an obstacle (fly over it); never by a runway |
| Haus, Kamin | house, chimney | `Steam::house_parts`, `HOUSE_WIDTH`, `CHIMNEY` | Level 4: small houses in the valley with a chimney the steam rises from; an obstacle |
| Blendung | glare | `Sunrise::glare`, `SUN_AT` | Level 4: the low sun blinds you for a moment; the screen washes out |
| Wrack | wreck | `Wreck` | A helicopter the plane flew into: it falls, burning, and is no obstacle any more |
| Ausleger | jib | `Crane`, `JIB_TOP` | The crane's long arm, on top of its mast |
| Mast | mast | `Crane`, `MAST_WIDTH` | The crane's lattice tower, from the jib down to the ground; it counts as a hit |
| Last, Stahlträger | load, steel beam | `Crane`, `LOAD_DROP` | What hangs from a crane's cable |
| Parallaxe | parallax | `Tower::size` | Things further away slide past more slowly |
| Möwe | gull | `Gull`, `gull-call.wav` | Sea bird at the beach: an obstacle; many call when they appear. One you hit tumbles to the ground (`Gull::knock_down`) |
| Melodie | tune | `Event::LevelComplete`, `new-level.wav` | Short rising tune when a level is landed and complete |
| Landung | landing | `Landing`, `World::landing` | How a level ends: approach on autopilot, touchdown, rollout, "Level complete" |
| Landebahn | runway (landing) | `Airport` | The runway a level ends on; the same kind as the takeoff runway |
| Reifenrauch | tyre smoke | `TyreSmoke`, `Event::MainWheelsTouch`, `Event::NoseWheelTouch` | A puff where a wheel meets the runway on landing: first the main wheels, then the nose wheel |
| Pistenbeleuchtung | runway lights | `RUNWAY_LIT_LEVELS`, `World::runway_lit` | Lights along the runway's edges and centre line, green at its start, red at its end; in levels 2 and 3 |
| Startwert | seed | `World::with_seed`, `Start::seed` | The number that decides the random weather and obstacles; for the leaderboard the server hands it out |

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
| Levelwechsel | level change | `level_view`, `LEVEL_DISTANCE` | After its distance a level ends with a landing, then the next begins |
| einblenden | fade in | `LevelView::fade` | No longer used between levels: the landing ends one, the next begins at once |
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
