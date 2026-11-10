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
| Hindernis | obstacle | — | Coming soon |
| Punkte | points | — | Coming soon |

## Phone and screen

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Handy | phone | — | Smartphone (iPhone or Android) |
| Querformat | landscape | `orientation: landscape` | Phone held sideways; the game's format |
| Hochformat | portrait | `orientation: portrait` | Phone held upright |
| quer drehen | turn sideways | — | Rotate the phone into landscape |
| Ausrichtungssperre | rotation lock | — | iPhone setting; Android: «Automatisch drehen» / "Auto-rotate" |
| Bewegung & Ausrichtung | Motion & Orientation | `DeviceMotionEvent` | iPhone permission needed for tilting |
| Home-Bildschirm | home screen | `manifest.webmanifest` | iPhone: «Zum Home-Bildschirm» / "Add to Home Screen" |
| Startbildschirm | home screen | `manifest.webmanifest` | Android: «Zum Startbildschirm hinzufügen» / "Add to Home screen" |
| Vollbild | full screen | `display: fullscreen` | Game fills the whole screen |
| Bildschirm | screen | — | |

## How the game works

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Regeln | rules | `crates/core` | Everything the game decides; no graphics, all tested |
| Spielschleife | game loop | `main` loop | Read input → ticks → draw, again and again |
| Tick | tick | `World::tick`, `TICK_SECONDS` | One fixed step of 1/60 second |
| Bild | frame | `next_frame()` | One drawn picture on the screen |
| zeichnen | draw | `crates/game` | Put the game on the screen |
| Eingabe | input | `Input` | What the player does (finger, keys, tilt) |
| Welt | world | `World` | Everything that exists in the game right now |

## Development

| Deutsch | English | In code | Meaning |
|---|---|---|---|
| Anleitung | manual | `docs/content` | This manual (player + developer part) |
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
**"Turn your phone sideways"**).
