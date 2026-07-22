# OLED Dash — ESP32-C3 Mini OLED

The ESP32-C3 Supermini OLED board is an incredibly cheap device. You can find them for less than €3 if you poke about on Temu or Aliexpress, especially if you buy several at once.

However, the tiny 72x40 pixel screen that is only 0.42 inches across the diagonal, and the presence of only one button that can be used for input limits what you can do with it.

Obviously, the best game to implement is ... a Geometry Dash-style **one-button** cube runner.

![`assets/demo.gif`](assets/demo.gif)

Tap the **BOOT** button (GPIO9) to jump. Die → tap to retry. Finish a level → tap to play the next.

## Install firmware in your browser

**[Install OLED Dash firmware](https://kf106.github.io/esp32-c3-supermini-oled-dasher/)**

Use **Chrome** or **Edge** on a computer. Plug the board in with USB, open the link, click **Install firmware**, pick the serial port, and wait. On Ubuntu the port is usually **`/dev/ttyACM0`**.

## Board

| Function | GPIO | Notes |
|----------|------|-------|
| I2C SDA | 5 | 400 kHz, onboard OLED |
| I2C SCL | 6 | onboard OLED |
| Blue LED | 8 | active-low — flashes 3× on level clear |
| BOOT button | 9 | active-low, internal pull-up — **jump / restart / next** |
| Display | — | SSD1306 `0x3C`, **72×40**, MONO_VLSB |

## Game

- **26 levels**, HUD shows **level number** (1–26) on the left, progress % on the right
- Terrain variety: flat (1–2, 9), 8px steps (3, 5, 7, 11–12, 15), hills (4, 6, 10, 14), mixed jumps + gradients (8, 13, 16), **gravity flips** (17–21: flip+revert; 22–24: 3 / 4 / 5 flips), **moving spikes** (25–26)
- Your original course is **difficulty 4**
- Completing a level **unlocks the next** and saves to flash — reset or power-cycle resumes at that level
- Clearing all twenty-one shows **SQUARE DASH COMPLETE!** — hold BOOT 2s on that screen (or the boot splash) to wipe back to level 1
- **SQUARE DASH** splash for **5 seconds** on boot — **hold BOOT for 2 seconds** to wipe progress (`ERASED`)

## Build and flash

From a terminal (Rust + `espflash`):

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

Or use the [browser installer](https://kf106.github.io/esp32-c3-supermini-oled-dasher/) above (no local toolchain).

## Layout

```
docs/flasher/          # browser installer (GitHub Pages)
firmware/src/
  main.rs      # OLED + BOOT + tick loop
  ssd1306.rs   # display driver
  framebuf.rs  # pixels / spikes / HUD
  font.rs      # 3×5 digits
  input.rs     # BOOT edge detect
  level.rs     # 16 courses + terrain
  game.rs      # physics + collisions
  save.rs      # flash-backed level progress
  led.rs       # blue LED celebrate
```

## License

[MIT](LICENSE.md)
