# Developing

The game is written in [Rust](https://www.rust-lang.org/) and runs in
the browser as WebAssembly. Drawing is done by the
[macroquad](https://macroquad.rs/) engine.

## What you need

- A computer with macOS, Linux or Windows
- [Rust](https://rustup.rs/) (install with `rustup`)
- [Git](https://git-scm.com/)
- Python 3 (for the local web server)
- Google Chrome (for automatic screenshots; optional)
- Linux only: the ALSA sound headers for `cargo run` (on Ubuntu:
  `sudo apt install libasound2-dev`)
- Only for level backgrounds: `rsvg-convert` and ImageMagick (macOS:
  `brew install librsvg imagemagick`)

## Set up

```sh
git clone https://github.com/m0nstergamez/airplane-scoring.git
cd airplane-scoring
rustup target add wasm32-unknown-unknown
```

## Run the game

| What | Command |
|---|---|
| Play in a window | `cargo run` |
| Play in the browser | `scripts/dev.sh`, then open <http://localhost:8080> |
| Run all checks | `scripts/check.sh` |
| Quick checks | `scripts/check.sh --fast` |
| Make the sound files again | `scripts/make-sounds.py` |
| Make the level backgrounds again | `scripts/make-backgrounds.sh` |
| Run the leaderboard server | `cargo run -p airplane-scoring-server` |
| Play against it | `scripts/dev.sh`, then open <http://localhost:8080/?board=http://localhost:8787> |

`scripts/dev.sh` keeps running: every time you save a file, the game is
rebuilt and the browser page reloads by itself:

```mermaid
flowchart LR
    S["You save a file"] --> W["watch.sh notices"] --> B{"Build OK?"}
    B -- yes --> L["Browser reloads"]
    B -- no --> E["Error in the terminal,<br/>browser keeps the last good version"]
```

## How the project is built

| Folder | What is in it |
|---|---|
| `crates/core` | The **rules** of the game: movement, steering, tilting, weather, crashes, sound volumes, scrolling. No graphics, no sound files. Everything here has tests. |
| `crates/game` | Draws the game, plays the sounds and reads keys, finger and sensors. No rules. |
| `crates/server` | The online leaderboard: hands out seeds, flies replays again with `core` and keeps the fastest times. Not part of the web page. |
| `web` | The web page that loads the game, the sound files in `web/sounds` and the level backgrounds in `web/levels`. |
| `art` | Drawings the game is made from: one folder per level in `art/levels`. |
| `scripts` | Helper commands (build, start, check). |
| `docs` | Notes and this manual. |

The most important rule: **game rules go into `core`, with a test first.**
`game` only passes on what the player does and draws what `core` says:

```mermaid
flowchart LR
    subgraph player["Player"]
        I["Finger, keys, tilt"]
    end
    subgraph game["crates/game: reads and draws"]
        R["Read input"]
        D["Draw"]
    end
    subgraph core["crates/core: rules and tests"]
        S["One steer value<br/>from -1 to +1"]
        W["World::tick<br/>moves the plane"]
    end
    I --> R --> S --> W --> D -- "screen" --> I
```

Every way of steering ends up as the same single number, `steer`. So
`core` does not care whether you tilt, drag or press keys – and adding a
new control means adding one more way to make that number.

### The game loop

The game calculates in fixed steps of 1/60 second, called **ticks**,
no matter how fast a device can draw:

```mermaid
flowchart TD
    A["New frame"] --> B["Read finger, keys, tilt"]
    B --> C{"Time for another tick?<br/>(one every 1/60 s)"}
    C -- yes --> T["World::tick:<br/>move the plane by steer"] --> C
    C -- no --> D["Draw the world"] --> A
```

A slow phone does several ticks per frame, a fast one sometimes none.
Because each tick is the same, the game behaves the same everywhere –
and a test can replay it exactly.

How many ticks a frame gets is decided by `Clock` in
`crates/core/src/clock.rs`. Until "Tap to fly" it gives none: the game
stands still on the start screen, and the flight (weather, levels)
begins with the tap.

### Sound

Sound follows the same split. `core` decides **what** you hear and
**how loud**; `game` only plays files:

```mermaid
flowchart LR
    subgraph core["crates/core: rules and tests"]
        W["World::tick"]
        M["Mix: one volume each for<br/>engine idle, engine power, wind, rain, beach"]
        E["Events:<br/>Crash, Lightning, Thunder,<br/>NewLevel, GullCall"]
    end
    subgraph game["crates/game: plays"]
        L["Loops play all the time,<br/>set their volume every frame"]
        O["Play a sound once<br/>per event"]
    end
    F["web/sounds/*.wav"]
    W --> M --> L
    W --> E --> O
    F --> L
    F --> O
```

- **Loops** (engine, wind, rain, beach) never stop. Each tick `core` computes a
  `Mix` with their volumes; the volumes move a little per tick, so
  nothing clicks.
- **The engine** is a turbine (jet) and two loops. macroquad 0.4.16
  cannot change the pitch of a sound, so climbing fades from the idle
  loop (lower whine, soft roar) to the power loop (higher whine, louder
  roar and hiss). Its volume follows the plane's distance
  (`Bounds::nearness`): full near the ground, `ENGINE_FAR_VOLUME` (30 %)
  at the top of the screen.
- **One-shot sounds** (thunder, crash, the tune for a new level, a gull's
  call) are events of one tick. Thunder is its own event because it arrives later
  than its lightning.
- **Sounds of a level** follow its landscape: the beach loop's volume is
  `BEACH_VOLUME` times how much of level 2 is on screen
  (`LevelView::presence`), so the waves fade in with the beach. The
  weather works the same way: rain may only be as heavy as the levels in
  `RAINY_LEVELS` are on screen (`LevelView::rain_room`). That list is
  empty for now, so levels 1 and 2 are dry; the weather keeps running
  underneath for later levels.
- **Hot-air balloons** (`crates/core/src/balloons.rs`) are level 1's
  obstacles: big, slow and rare, since level 1 is the easiest level.
  Each one takes off from a launch pad (`Pad`: it slides in standing on
  it and lifts off at `TAKEOFF_AT`), lands on one (touching down at
  `TOUCHDOWN_AT`), comes level at the plane's height, or sinks past
  from it (`BalloonWay`; `Balloon::at_plane_y`, up to the very top). The ways take turns: a shuffled round of all four,
  never the same way twice in a row. Launch pads are obstacles too. Core spawns them (seeded), moves them and checks
  the crash against the envelope and the basket; a `BalloonBurner` event
  plays its sound as it appears or takes off.
- **Huts** (`crates/core/src/huts.rs`) stand big in level 1's foreground
  grass at the bottom of the screen, rare, sliding with the landscape.
  They belong to the landscape, so they grow with `Bounds::scale` but not
  with `OBJECT_SIZE`; so do the launch pads, which lie next to them.
  Huts and launch pads wait for each other, so a pad never lies on a hut.
- **Depth** (`Bounds::depth(y)`): what flies (balloons, gulls) is
  smallest (`OBJECT_SIZE`) at the top of the screen and grows steadily
  all the way down to full size at a balloon standing on a pad. The
  plane follows `Bounds::plane_depth(y)`: as small at the top, growing to
  `PLANE_NEAR_SIZE` (2.4) near the ground. Drawing and crashes use the
  same factor, so a plane flying low is bigger and easier to hit.
- **Crashes use the drawn shapes** (`crates/core/src/shape.rs`): the
  plane (`Player::shape`: fuselage, nose, wing, tail, cockpit) and each
  obstacle (`Hut::shape`, `Pad::shape`, `Balloon::shape`,
  `Gull::shape`) are a few convex polygons with the same points the
  game crate draws. Only a real overlap is a crash; a near miss, or the
  empty corner beside a roof, is not.
- **Lifeguard towers** (`crates/core/src/towers.rs`) stand in the
  beach's sand: rare (one every 6–10 s) and not tall (92–112 px). Each
  has its own depth (`Tower::near`): by the water it stands higher
  (`TOWER_GROUND`), is `TOWER_FAR_SIZE` (55 %) as big and slides that much
  slower (parallax); at the front it is full size. Far ones are drawn
  first. `Tower::parts` places legs, ladder,
  platform, cabin and roof; crashes (`Tower::shape`) and drawing both use
  it.
- **The town** (`crates/core/src/city.rs`, level 3, `CITY_LEVEL`)
  sends high-rises and tower cranes, one every 3–5 s
  (`CITY_GAP_SECONDS`) with at least `CITY_ROOM` (140 px) of free ground
  between them; `CRANE_SHARE` (40 %) are cranes, never two in a row and
  at the latest after `HIGH_RISES_IN_A_ROW` (2) high-rises.
  A `HighRise` stands at the front (walls, stair house, antenna): fly
  over it. A `Crane` stands on the ground too: its mast runs from the
  jib (`JIB_TOP`) down to the ground, a load hangs on a cable
  (`LOAD_DROP`), and all of it is hit: fly over it. Every 5–8 s a
  `FlyingHeli` comes at the plane at its height, never lower than
  `FLYING_HELI_LOWEST` (above the town), so the top is not safe either.
  `parts` places everything; crashes (`shape`) and drawing use it. A
  helicopter the plane hits (flying or parked, `heli_shape`) becomes a
  falling, burning `Wreck` (`WRECK_SECONDS`) and sends
  `Event::HeliCrash` (the explosion sound).
  `HELI_SHARE` (35 %) of the high-rises have a `Heli`: it takes off once
  its high-rise is at `HELI_TAKEOFF_AT` and climbs away back and up, or
  lands by `HELI_TOUCHDOWN_AT` (its sink rate allows for the game's
  speed). Its place is kept relative to the roof, so it slides with it.
- **The sunrise** (`crates/core/src/sunrise.rs`, level 4,
  `SUNRISE_LEVEL`): about every two seconds (`GOOSE_GAP_SECONDS`) a V of geese
  comes at the plane, its tip or an arm at the plane's height
  (`Goose::shape`, knocked down like gulls, `Event::GooseHonk` with one of
  `GOOSE_CALLS` calls, never the same twice in a row). Each `Steam` is a
  small house (`house_shape`, solid) with a column rising from its
  chimney; inside the column the plane is pushed up (`STEAM_LIFT`,
  `Event::SteamHiss`), no life lost. `FarHighRise`s stand in the valley,
  solid (`shape`: walls, roof, stair house, mast); like everything else
  they only come while no runway is on screen. `Sunrise::glare`
  (0..1, every 10–16 s for `GLARE_SECONDS`) tells the game how much to
  wash the screen in the light of the sun, which it draws at `SUN_AT`.
- **Gulls** (`crates/core/src/gulls.rs`) are the beach's obstacles. Core
  decides when and where they appear (seeded, like the weather), moves
  them and checks the crash; a gull that calls sends a `GullCall` event,
  so every call you hear belongs to a gull you see.
- **Struck gulls** (`Gull::knock_down`): a gull the plane flies into
  is thrown up a little, then falls (`GULL_FALL`) and tumbles
  (`Gull::tumble`); it is no obstacle any more and is dropped below the
  screen. The game only turns the drawing by `tumble`.
- **Lives** are counted in core too: a crash costs one (`World.lives`),
  a recovering plane is safe for `SAFE_SECONDS` (`Player::is_safe`), and
  every lost life adds smoke (`World::smoke`, `DAMAGE_SMOKE`); when the
  last one goes the plane explodes (`Event::Explosion`, `World::explosion`,
  `World::explosion_progress` over the crash time), then `Event::GameOver` fires and
  `World::tick` stands still (`World::is_over`). The game crate only draws
  it and, once `World::can_restart` says so, starts a new `World` on a tap.
  The time on screen (`World::seconds_in_level`, shown by `clock_text`
  as "m:ss") follows from the ticks flown in this level, so it stops
  with them at game over and starts at 0:00 in every level;
  `World::seconds_flown` counts the whole flight for the speed-up.
  Only "Level complete" records a best time: `World::millis_in_level`
  (shown by `best_time_text` as "m:ss.mmm") goes into `BestTimes`,
  fastest first; a game over or leaving records nothing.
- **Weather is random but repeatable:** `World::with_seed` takes a seed;
  the same seed gives the same rain and the same lightning, so tests can
  replay it.
- **The sound files are made by a script**, not recorded:
  `scripts/make-sounds.py` computes them from noise and pulses. Change
  the script, run it, commit the new `.wav` files.

### Level backgrounds

Each level has a landscape behind the plane. It starts as a photo and
ends as a picture the game repeats side by side while it scrolls:

```mermaid
flowchart LR
    P["Photo<br/>art/levels/level1/photo.jpg<br/>(stays on your computer)"] --> S["Drawing<br/>art/levels/level1/background.svg"]
    S --> M["scripts/make-backgrounds.sh"] --> G["Picture<br/>web/levels/level1.png"]
    G --> D["Game: copies side by side,<br/>sliding left"]
```

Every level has its own folder with the same file names:

```text
art/levels/
  level1/
    photo.jpg         the photo (not committed)
    background.svg    the drawing (committed)
  level2/
  level3/
```

A new level in six steps:

1. Make the folder, for example `art/levels/level2/`.
2. Put your photo in it as `photo.jpg`. Take a photo held sideways,
   without people. It is never committed: the repository is public and
   phone photos contain the place where they were taken.
3. Draw `background.svg` from the photo, or ask Claude Code: "Make
   `art/levels/level2/background.svg` from `photo.jpg`, like level 1."
   Copy `art/levels/level1/background.svg` as a start: its comments
   explain the rules below.
4. Run `scripts/make-backgrounds.sh level2`. It makes
   `web/levels/level2.png` and checks the seam.
5. Raise `LEVEL_COUNT` in `crates/core/src/level.rs` by one, so the game
   shows the new level.
6. Look at the picture, then commit the SVG and the PNG.

The SVG has to follow a few rules, because the right edge of one copy
touches the left edge of the next:

- **Size:** `viewBox="0 0 1600 740"`, about the shape of a phone held
  sideways. The game scales it to the screen height.
- **Flat colours, calm and a bit pale**, so the plane and the white text
  stand out.
- **Things that span the whole width** (sky, water, road, grass) only
  change from top to bottom, never from left to right.
- **Everything else is drawn three times** (at x = −1600, 0 and +1600).
  A cloud that sticks out on the right comes back in on the left.
- **One group per layer** (sky, hills, water, shore, road, grass,
  fence). Later the layers can scroll at different speeds.

If the edges do not match, `make-backgrounds.sh` stops with "seam too
visible". The game itself only knows how far the landscape has moved,
`background_x` in `crates/core/src/scenery.rs`, and draws as many copies
as the screen needs:

```mermaid
flowchart LR
    T["Ticks so far"] --> D["scroll_distance:<br/>ticks × SCROLL_SPEED"]
    D --> X["background_x:<br/>where copy 1 starts (0 to −width)"]
    X --> C["Draw copies side by side<br/>until the screen is full"]
```

A level ends with a landing (`World::landing`, `Landing` in
`crates/core/src/lib.rs`): after `LEVEL_DISTANCE` of landscape (`LEVEL_SECONDS` at cruising speed, sooner with more throttle) the landing
runway comes in, the autopilot flies the approach and touches down, the
plane brakes to a stop, "Level N complete" shows (a tap goes on), the
next level's landscape takes over at once and counts down, and the
plane takes off from its runway. `CLEAR_SECONDS` before the landing no
new obstacles come. To start directly in a level, add `?level=2` to the
address (`LEVEL=2 scripts/shot.sh` for a screenshot).

```mermaid
flowchart LR
    T["Takeoff"] --> F["Flying,<br/>LEVEL_SECONDS"] --> A["Approach:<br/>autopilot, gear down"] --> R["Touchdown,<br/>rollout"] --> C["Level complete"] --> N["Next level:<br/>countdown 3-2-1"] --> T
```

### Leaderboard

The online leaderboard ranks the same thing as the best times: the time
of a completed level, fastest first, one board per level. The game
cannot be trusted with a time (anyone can change a web page), so the
server never takes one. It hands out a **seed** for each flight; the
game writes down every input (`crates/core/src/replay.rs`, the
**replay**); at "Level complete" the server flies that replay again with
the same `core` and reads the time itself. That works because
`World::tick` is deterministic: same seed, same inputs, same flight.

```mermaid
sequenceDiagram
    participant P as web/index.html (board plugin)
    participant G as game (board.rs)
    participant S as server (crates/server)
    P->>S: POST /v1/runs (start level)
    S-->>P: run id and seed
    G->>P: takes the seed before the first tick
    Note over G: every tick: the input is written down
    G->>P: Level complete: the replay so far
    P->>S: POST /v1/runs/{id}/replay
    S->>S: flies it again with core, checks the time it had
    S-->>P: level, time, rank
    P-->>G: World rank 3 of 120
```

Pilots are anonymous: the server gives each browser a secret token and
keeps only its hash. The leaderboard is off while the `board-api` meta
tag in `web/index.html` is empty; then the game plays as before. Every
change to `core` that changes how a flight goes also changes replays:
the server must be deployed with the same `core` as the published game.

The server's settings come from the environment: `PORT` (8787),
`DB_PATH` (`leaderboard.sqlite`, an SQLite file) and `ALLOWED_ORIGINS`
(the pages that may call it, comma separated). GitHub Pages only serves
files, so the server runs elsewhere.

Before it goes online, the **stage** tests the whole chain on your Mac:
an [OrbStack](https://orbstack.dev/) Linux machine named `stage` (amd64,
like Fly's machines) runs the real server image from the `Dockerfile`
and serves the game with `board-api` pointing at it. So the browser's
wasm makes the replays and an x86-64 Linux server flies them again, as
in production. It needs OrbStack; `e2e` also needs
[wasmtime](https://wasmtime.dev/).

| What | Command |
|---|---|
| Create, build and start the stage | `scripts/stage.sh up`, then open <http://localhost:8180> |
| Check it automatically (about a minute) | `scripts/stage.sh e2e` |
| The server's log | `scripts/stage.sh logs` |
| Empty its database | `scripts/stage.sh reset` |
| Stop it / remove the machine | `scripts/stage.sh down` / `scripts/stage.sh delete` |

`e2e` signs up a pilot, gets a seed, lets core's demo pilot (built as
wasm) land level 1, waits until that much real time has passed and
checks the server reads the same time and puts it on the board. After a
change to `core`, `crates/server` or the page, run `up` again.

Then the server goes online, for example on
[Fly.io](https://fly.io/) (`Dockerfile`, `fly.toml`; no free tier).
To put it there once:

1. Install `flyctl` and log in (`fly auth login`).
2. Change `app` in `fly.toml` to a name nobody has taken yet.
3. Check the region with `fly platform regions` (`fra` in `fly.toml`).
4. Run `fly launch --no-deploy --copy-config` and say no to databases.
5. Run `fly volumes create leaderboard_data --size 1 --region fra`.
6. Run `fly deploy`, then `fly scale count 1`: a second machine would
   have its own, empty database.
7. Put `https://<app>.fly.dev` into the `board-api` meta tag and open a
   pull request.

After a change to `core` or `crates/server`, run `fly deploy` again.

### Test first

New rules are written in three steps:

```mermaid
flowchart LR
    R["1. Write a test<br/>it fails: red"] --> G["2. Write the code<br/>the test passes: green"]
    G --> C["3. Tidy up<br/>tests stay green"] --> R
```

The failing test proves that the test really checks the new rule. If it
passed right away, it would test nothing new.

## Your first change

Make the plane climb and dive faster:

1. Open `crates/core/src/lib.rs`.
2. Find `PLAYER_CLIMB_SPEED` and change `300.0` to `500.0`.
3. Save. If `scripts/dev.sh` is running, the browser reloads – try it.
4. Run `scripts/check.sh`. If a test fails, read its message: tests
   describe how the game should behave.

More numbers you can try. Sizes and speeds in pixels are for a phone
in landscape (`DESIGN_HEIGHT`, 390 px high); on a bigger screen the game
multiplies them by `Bounds::scale` (screen height / 390), so everything
keeps its size relative to the landscape:

| Constant | File | Meaning |
|---|---|---|
| `DESIGN_HEIGHT` | `crates/core/src/lib.rs` | the screen height all pixel sizes and speeds are made for |
| `PLANE_NEAR_SIZE` | `crates/core/src/lib.rs` | how big the plane gets near the ground (1 = first design) |
| `OBJECT_SIZE` | `crates/core/src/lib.rs` | how big the plane and obstacles are drawn and hit (1 = first design, now 0.41): the size up high; smaller leaves more room |
| `PLAYER_CLIMB_SPEED` | `crates/core/src/lib.rs` | climb/dive speed (pixels per second) |
| `CLIMB_RESPONSE` | `crates/core/src/lib.rs` | how quickly the plane takes up a climb or dive (smaller = smoother) |
| `MAX_PITCH_DEG` | `crates/core/src/lib.rs` | how far the nose tilts at full climb or dive (degrees) |
| `PLAYER_X_FRACTION` | `crates/core/src/lib.rs` | where the plane sits (0.2 = 20 % from the left) |
| `TILT_FULL_DEG` | `crates/core/src/tilt.rs` | how far to turn the phone for full speed |
| `TILT_DEAD_ZONE_DEG` | `crates/core/src/tilt.rs` | small turns that are ignored |
| `TILT_SMOOTHING` | `crates/core/src/tilt.rs` | smoother (lower) or more direct (higher) tilting |
| `CRASH_SECONDS` | `crates/core/src/lib.rs` | how long a crashed plane is out of control |
| `LIVES` | `crates/core/src/lib.rs` | lives at the start of a flight |
| `SPEED_PER_SECOND` | `crates/core/src/lib.rs` | how much faster the game gets per second flown (`World::speed`, at most `MAX_SPEED`) |
| `AIRSPEED_RANGE` | `crates/core/src/lib.rs` | slowest and fastest airspeed you can set (1 = cruising) |
| `OVERSPEED_AT` | `crates/core/src/lib.rs` | airspeed above which "Overspeed" warns; after `OVERSPEED_SECONDS` it breaks apart |
| `STALL_AT` | `crates/core/src/lib.rs` | airspeed below which "Pull up" warns and the plane sags (up to `STALL_SINK`) |
| `ROLL_SECONDS` | `crates/core/src/airport.rs` | how long the plane rolls down the runway before it lifts off |
| `SHAKE_PX` | `crates/core/src/lib.rs` | how far the screen shakes at the explosion (`World::shake`) |
| `SAFE_SECONDS` | `crates/core/src/lib.rs` | how long the plane blinks and cannot crash after a crash |
| `RAIN_SECONDS` | `crates/core/src/weather.rs` | shortest and longest rain |
| `STORM_RAIN` | `crates/core/src/weather.rs` | how heavy rain must be for lightning (0 to 1) |
| `STRIKE_GAP_SECONDS` | `crates/core/src/weather.rs` | time between two lightning strikes |
| `ENGINE_VOLUME` | `crates/core/src/sound.rs` | how loud the engine is (0 to 1) |
| `BEACH_VOLUME` | `crates/core/src/sound.rs` | how loud the waves and gulls are (0 to 1) |
| `GULL_GAP_SECONDS` | `crates/core/src/gulls.rs` | shortest and longest time between two gulls |
| `GULL_SPEED` | `crates/core/src/gulls.rs` | how fast gulls fly (pixels per second) |
| `TOWER_GAP_SECONDS` | `crates/core/src/towers.rs` | shortest and longest time between two lifeguard towers |
| `TOWER_HEIGHT` | `crates/core/src/towers.rs` | lowest and highest lifeguard tower (pixels) |
| `BALLOON_GAP_SECONDS` | `crates/core/src/balloons.rs` | shortest and longest time between two balloons |
| `CITY_GAP_SECONDS` | `crates/core/src/city.rs` | shortest and longest time between two high-rises or cranes |
| `CRANE_SHARE` | `crates/core/src/city.rs` | how likely the next one is a crane (0 to 1) |
| `BALLOON_RISE` | `crates/core/src/balloons.rs` | how fast balloons rise (pixels per second) |
| `HUT_GAP_SECONDS` | `crates/core/src/huts.rs` | shortest and longest time between two huts |
| `HUT_HEIGHT` | `crates/core/src/huts.rs` | lowest and highest hut (pixels) |
| `SCROLL_SPEED` | `crates/core/src/scenery.rs` | how fast the landscape slides past (pixels per second) |
| `LEVEL_SECONDS` | `crates/core/src/level.rs` | how long one level lasts at cruising speed (seconds); sets `LEVEL_DISTANCE` |
| `CLEAR_SECONDS` | `crates/core/src/airport.rs` | how long before a landing no new obstacles come |
| `BRAKE_SECONDS` | `crates/core/src/airport.rs` | how long the plane brakes to a stop after touchdown |

## Share your change

Changes go into the game through a **pull request** on GitHub:

```mermaid
flowchart LR
    B["Branch"] --> C["Commit"] --> P["Pull request"] --> K{"Checks green?"}
    K -- no --> C
    K -- yes --> M["Merge"] --> D["GitHub Pages"] --> O["Online for everyone"]
```

1. Make a new branch: `git switch -c my-change`
2. Commit: `git commit -am "Faster plane"`
3. Push and open a pull request: `git push -u origin my-change`, then
   follow the link GitHub shows.
4. GitHub runs all checks automatically. When they are green and the
   pull request is merged, the new version is online a minute later.

If your change affects what players see or do, update this manual too –
in **both** languages.
