# OLED Dash — ESP32-C3 Mini OLED

The ESP32-C3 Supermini OLED board is an incredibly cheap device. You can find them for less than €3 if you poke about on Temu or Aliexpress, especially if you buy several at once.

However, the tiny 72x40 pixel screen that is only 0.42 inches across the diagonal, and the presence of only one button that can be used for input limits what you can do with it.

Obviously, the best game to implement is ... a Geometry Dash-style **one-button** cube runner.

![`assets/demo.gif`](assets/demo.gif)

Tap the **BOOT** button (GPIO9) to jump. Die → tap to retry. Finish a level → tap to play the next.

## Board

| Function | GPIO | Notes |
|----------|------|-------|
| I2C SDA | 5 | 400 kHz, onboard OLED |
| I2C SCL | 6 | onboard OLED |
| BOOT button | 9 | active-low, internal pull-up — **jump / restart / next** |
| Display | — | SSD1306 `0x3C`, **72×40**, MONO_VLSB |

## Game

- **16 levels**, difficulty **1–16** (HUD shows difficulty on the left, progress % on the right)
- Terrain variety: flat (1–2, 9), 8px steps (3, 5, 7, 11–12, 15), hills (4, 6, 10, 14), mixed jumps + gradients (8, 13, 16)
- Your original course is **difficulty 4**
- Completing a level **unlocks the next** and saves to flash — unplug and plug back in to resume there
- Clearing all sixteen shows **SQUARE DASH COMPLETE!** — tap BOOT to replay level 16
- **SQUARE DASH** splash for 3 seconds on boot

### Splash

![`assets/splash-72x40.png`](assets/splash-72x40.png)

## Course maps

PNGs live in [`assets/courses/`](assets/courses/). Regenerate with:

```bash
python3 firmware/tools/gen_course_maps.py
```

| Level | File |
|------:|------|
| 1 | [`level-01.png`](assets/courses/level-01.png) |
| 2 | [`level-02.png`](assets/courses/level-02.png) |
| 3 | [`level-03.png`](assets/courses/level-03.png) |
| 4 | [`level-04.png`](assets/courses/level-04.png) |
| 5 | [`level-05.png`](assets/courses/level-05.png) |
| 6 | [`level-06.png`](assets/courses/level-06.png) |
| 7 | [`level-07.png`](assets/courses/level-07.png) |
| 8 | [`level-08.png`](assets/courses/level-08.png) |
| 9 | [`level-09.png`](assets/courses/level-09.png) |
| 10 | [`level-10.png`](assets/courses/level-10.png) |
| 11 | [`level-11.png`](assets/courses/level-11.png) |
| 12 | [`level-12.png`](assets/courses/level-12.png) |
| 13 | [`level-13.png`](assets/courses/level-13.png) |
| 14 | [`level-14.png`](assets/courses/level-14.png) |
| 15 | [`level-15.png`](assets/courses/level-15.png) |
| 16 | [`level-16.png`](assets/courses/level-16.png) |

## Build and flash

```bash
cd firmware
cargo build --release
./flash.sh
```

Optional:

```bash
PORT=/dev/ttyACM0 ./flash.sh
MONITOR=1 ./flash.sh
```

## Layout

```
firmware/src/
  main.rs      # OLED + BOOT + tick loop
  ssd1306.rs   # display driver
  framebuf.rs  # pixels / spikes / HUD
  font.rs      # 3×5 digits
  input.rs     # BOOT edge detect
  level.rs     # 16 courses + terrain
  game.rs      # physics + collisions
  save.rs      # flash-backed level progress
```

## License

[MIT](LICENSE.md)
