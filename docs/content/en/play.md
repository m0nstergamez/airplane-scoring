# Playing

## Start

1. Open <https://m0nstergamez.github.io/airplane-scoring/> on your phone.
2. Turn your phone **sideways** (landscape). Held upright, the game asks
   you to turn it.
3. Tap **"Tap to fly"**. On an iPhone, allow access to "Motion & Orientation"
   when asked.

On a computer the game starts right away.

## Controls

You have three ways to steer. If you use several at once, the finger
wins, then the keys, then tilting:

```mermaid
flowchart TD
    F{"Finger on the screen?"} -- yes --> FS["Fly to the finger's height"]
    F -- no --> K{"Key pressed?"}
    K -- yes --> KS["Climb or dive"]
    K -- no --> T{"Tilting active?"}
    T -- yes --> TS["Steer by tilting"]
    T -- no --> L["Fly level"]
```

### Tilt (phone)

Hold your phone like a steering wheel. **The phone is the plane:**

- Turn the right side **up** → the plane climbs.
- Turn the right side **down** → the plane dives.
- Hold it still → the plane flies level.

The position you hold the phone in when you tap "Tap to fly" counts as
"level". Small wobbles are ignored; turning about a quarter of the way
to vertical (25°) is full speed:

```mermaid
%%{init: {"themeVariables": {"xyChart": {"plotColorPalette": "#1f6feb"}}}}%%
xychart-beta
    title "How far you turn → how strongly you steer"
    x-axis "Turn in degrees (minus = right side up)" -30 --> 30
    y-axis "Steering (-1 = full climb, +1 = full dive)" -1 --> 1
    line [-1, -1, -1, -1, -1, -1, -0.96, -0.91, -0.87, -0.83, -0.78, -0.74, -0.7, -0.65, -0.61, -0.57, -0.52, -0.48, -0.43, -0.39, -0.35, -0.3, -0.26, -0.22, -0.17, -0.13, -0.09, -0.04, 0, 0, 0, 0, 0, 0.04, 0.09, 0.13, 0.17, 0.22, 0.26, 0.3, 0.35, 0.39, 0.43, 0.48, 0.52, 0.57, 0.61, 0.65, 0.7, 0.74, 0.78, 0.83, 0.87, 0.91, 0.96, 1, 1, 1, 1, 1, 1]
```

Between −2° and +2° nothing happens (so a calm hand flies level), from
there steering grows evenly, and from 25° on it is at full strength.

### Finger (phone and computer)

Touch the screen and hold: the plane flies to the height of your finger.
On a computer, hold down the mouse button instead.

### Keys (computer)

| Key | Action |
|---|---|
| ↑ or W | climb |
| ↓ or S | dive |

## Install on your phone

Add the game to your home screen. It then opens full screen like an app.

### iPhone (Safari)

1. Tap the share button (square with an arrow).
2. Choose **"Add to Home Screen"**.

### Android (Chrome)

1. Tap the menu **⋮** at the top right.
2. Choose **"Add to Home screen"** or **"Install app"**.

## Help

### Tilting does nothing

- Did you tap **"Tap to fly"**? Tilting only starts after that.
- iPhone: did you allow "Motion & Orientation"? If you said no, reload
  the page and tap "Tap to fly" again. If the question does not come
  back, close Safari completely and open the game again.
- Is the phone lying flat on the table? Tilting needs the phone held up.
- You can always steer with your finger instead.

### The plane drifts by itself

You held the phone at a different angle when you started. Turn the
phone the other way round (landscape) and back, or reload the page:
your current position then becomes "level" again.

### The game shows "Turn your phone sideways"

The game is made for landscape. Turn your phone. If it does not rotate,
switch off the rotation lock.
