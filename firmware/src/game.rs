//! Geometry Dash–style cube runner (one-button jump).

use crate::framebuf;
use crate::level::{self, ground_at, Kind, Level, TerrainMode, LEVEL_COUNT};
use crate::save;
use crate::splash;
use crate::ssd1306::{FRAME_LEN, HEIGHT, WIDTH};

const CUBE: i32 = 5;
const PLAYER_SX: i32 = 14;

const GRAVITY: i32 = 1;
const JUMP_VEL: i32 = -5;
const SCROLL: i32 = 2;

const SPIKE_W: i32 = 6;
const SPIKE_H: i32 = 6;
const BLOCK_W: i32 = 6;
const BLOCK_H: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Playing,
    Dead { timer: u8 },
    Won { timer: u8 },
    /// Finished level 16 — show complete splash until BOOT.
    Complete { timer: u8 },
}

pub struct Game {
    level_index: u8,
    scroll: i32,
    y: i32,
    vy: i32,
    on_ground: bool,
    phase: Phase,
    attempts: u32,
}

impl Game {
    pub fn new(saved_level: u8) -> Self {
        let level_index = saved_level.min((LEVEL_COUNT - 1) as u8);
        let mut g = Self {
            level_index,
            scroll: 0,
            y: 0,
            vy: 0,
            on_ground: true,
            phase: Phase::Playing,
            attempts: 1,
        };
        g.reset_run();
        g
    }

    fn level(&self) -> &'static Level {
        level::get(self.level_index)
    }

    fn player_world_x(&self) -> i32 {
        self.scroll + PLAYER_SX
    }

    /// Dirt ground under the cube, or a block top if we're on / jumping onto it.
    fn floor_at_player(&self) -> i32 {
        let lvl = self.level();
        let px = self.player_world_x();
        let mut floor = ground_at(lvl, px + CUBE / 2);
        let feet = self.y + CUBE;

        for obs in lvl.obstacles {
            if obs.kind != Kind::Block {
                continue;
            }
            let ox = obs.x;
            if px + CUBE <= ox || px >= ox + BLOCK_W {
                continue;
            }
            let top = ground_at(lvl, ox + BLOCK_W / 2) - BLOCK_H;
            // Airborne: block is a platform (jump onto / over).
            // Grounded: only if already on top — walking into the face stays lethal.
            if !self.on_ground || feet <= top + 3 {
                if top < floor {
                    floor = top;
                }
            }
        }
        floor
    }

    fn reset_run(&mut self) {
        self.scroll = 0;
        self.vy = 0;
        self.on_ground = true;
        self.phase = Phase::Playing;
        self.y = self.floor_at_player() - CUBE;
    }

    fn progress_pct(&self) -> u32 {
        let len = self.level().length.max(1);
        ((self.scroll * 100) / len).min(100) as u32
    }

    pub fn update(&mut self, jump_pressed: bool) {
        match self.phase {
            Phase::Dead { timer } => {
                if timer > 0 {
                    self.phase = Phase::Dead { timer: timer - 1 };
                } else if jump_pressed {
                    self.attempts = self.attempts.saturating_add(1);
                    self.reset_run();
                }
                return;
            }
            Phase::Won { timer } => {
                if timer > 0 {
                    self.phase = Phase::Won { timer: timer - 1 };
                } else if jump_pressed {
                    self.advance_after_win();
                }
                return;
            }
            Phase::Complete { timer } => {
                if timer > 0 {
                    self.phase = Phase::Complete { timer: timer - 1 };
                } else if jump_pressed {
                    // Replay the final level.
                    self.attempts = 1;
                    self.reset_run();
                }
                return;
            }
            Phase::Playing => {}
        }

        // Scroll first so a jump on the frame a block arrives can still clear it.
        self.scroll += SCROLL;

        if self.scroll >= self.level().length {
            self.on_level_complete();
            return;
        }

        if jump_pressed && self.on_ground {
            self.vy = JUMP_VEL;
            self.on_ground = false;
        }

        // Only integrate vertical motion while airborne. Grounded frames would
        // otherwise sink 1px into the dirt every tick.
        if !self.on_ground {
            self.vy += GRAVITY;
            if self.vy > 4 {
                self.vy = 4;
            }
            self.y += self.vy;
        } else {
            self.vy = 0;
        }

        self.resolve_terrain();

        if self.hits_hazard() || self.buried_in_terrain() {
            self.phase = Phase::Dead { timer: 40 };
        }
    }

    fn on_level_complete(&mut self) {
        // Persist the level you'll play next (survives unplug on the win screen).
        if (self.level_index as usize) + 1 < LEVEL_COUNT {
            save::save_level(self.level_index + 1);
            self.phase = Phase::Won { timer: 70 };
        } else {
            save::save_level(self.level_index);
            self.phase = Phase::Complete { timer: 90 };
        }
    }

    fn advance_after_win(&mut self) {
        if (self.level_index as usize) + 1 < LEVEL_COUNT {
            self.level_index += 1;
        }
        self.attempts = 1;
        self.reset_run();
    }

    fn resolve_terrain(&mut self) {
        let g = self.floor_at_player();
        let feet = self.y + CUBE;

        if !self.on_ground {
            // Jumping into a block face: catch onto the top instead of dying.
            if feet > g && self.player_over_block_platform(g) {
                self.y = g - CUBE;
                self.vy = 0;
                self.on_ground = true;
                return;
            }
            if self.vy >= 0 && feet >= g {
                self.y = g - CUBE;
                self.vy = 0;
                self.on_ground = true;
            }
            return;
        }

        let rise = feet - g;
        if self.level().mode == TerrainMode::Stepped {
            if g > feet {
                self.on_ground = false;
            } else if rise <= 0 {
                self.y = g - CUBE;
            }
            // rise > 0: ledge face — buried_in_terrain handles death
        } else {
            // Follow gentle slopes; refuse steep faces (jump pairs on 8/13/16).
            let g_prev = ground_at(
                self.level(),
                self.scroll.saturating_sub(3) + PLAYER_SX + CUBE / 2,
            );
            let slope_up = g_prev - g;
            if slope_up <= 2 && rise <= 3 {
                self.y = g - CUBE;
            } else if g > feet {
                self.on_ground = false;
            }
        }
    }

    /// True when `platform_y` is a block top under the player (not plain dirt).
    fn player_over_block_platform(&self, platform_y: i32) -> bool {
        let lvl = self.level();
        let px = self.player_world_x();
        for obs in lvl.obstacles {
            if obs.kind != Kind::Block {
                continue;
            }
            let ox = obs.x;
            if px + CUBE <= ox || px >= ox + BLOCK_W {
                continue;
            }
            let top = ground_at(lvl, ox + BLOCK_W / 2) - BLOCK_H;
            if top == platform_y {
                return true;
            }
        }
        false
    }

    fn buried_in_terrain(&self) -> bool {
        // Only dirt ledges count — block platforms are handled via floor_at_player.
        let dirt = ground_at(self.level(), self.player_world_x() + CUBE / 2);
        let feet = self.y + CUBE;
        let rise = feet - dirt;
        if self.on_ground {
            // Standing on a block: feet well above dirt — not buried.
            if feet + 2 < dirt {
                return false;
            }
            return match self.level().mode {
                TerrainMode::Stepped => rise > 0,
                TerrainMode::Smooth => rise > 3,
            };
        }
        rise > 4 && self.y < dirt
    }

    fn hits_hazard(&self) -> bool {
        let px = self.player_world_x();
        let py = self.y;
        let feet = py + CUBE;
        let lvl = self.level();
        for obs in lvl.obstacles {
            match obs.kind {
                Kind::Spike => {
                    let floor = ground_at(lvl, obs.x + SPIKE_W / 2);
                    let sx = obs.x;
                    let sy = floor - SPIKE_H;
                    if aabb(px, py, CUBE, CUBE, sx + 1, sy + 1, SPIKE_W - 2, SPIKE_H - 1) {
                        return true;
                    }
                }
                Kind::Block => {
                    let floor = ground_at(lvl, obs.x + BLOCK_W / 2);
                    let bx = obs.x;
                    let top = floor - BLOCK_H;
                    if !aabb(px, py, CUBE, CUBE, bx, top, BLOCK_W, BLOCK_H) {
                        continue;
                    }
                    // Top is a platform — only the face / body is lethal.
                    if feet <= top + 3 {
                        continue;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn draw(&self, frame: &mut [u8; FRAME_LEN]) {
        if matches!(self.phase, Phase::Complete { .. }) {
            splash::draw_complete(frame);
            return;
        }

        framebuf::clear(frame);
        let lvl = self.level();

        // Variable-height ground (column by column). Checker hatch reads as
        // "stripes" on this panel; solid fill is too heavy.
        for sx in 0..WIDTH as i32 {
            let g = ground_at(lvl, self.scroll + sx);
            if g < HEIGHT as i32 {
                framebuf::set_pixel(frame, sx, g, true);
            }
            for y in (g + 1)..HEIGHT as i32 {
                if ((sx + y + self.scroll) & 2) == 0 {
                    framebuf::set_pixel(frame, sx, y, true);
                }
            }
        }

        for obs in lvl.obstacles {
            let sx = obs.x - self.scroll;
            if sx < -10 || sx > WIDTH as i32 + 2 {
                continue;
            }
            match obs.kind {
                Kind::Spike => {
                    let floor = ground_at(lvl, obs.x + SPIKE_W / 2);
                    framebuf::fill_spike_up(frame, sx, floor - 1, SPIKE_W, SPIKE_H);
                }
                Kind::Block => {
                    let floor = ground_at(lvl, obs.x + BLOCK_W / 2);
                    framebuf::fill_rect(frame, sx, floor - BLOCK_H, BLOCK_W, BLOCK_H);
                }
            }
        }

        let show_cube = match self.phase {
            Phase::Dead { timer } => (timer & 2) == 0,
            _ => true,
        };
        if show_cube {
            framebuf::fill_rect(frame, PLAYER_SX, self.y, CUBE, CUBE);
            framebuf::set_pixel(frame, PLAYER_SX + 3, self.y + 1, false);
        }

        // HUD: difficulty (1-16) left, progress % right
        framebuf::draw_score(frame, 11, 0, u32::from(lvl.difficulty));
        framebuf::draw_score(frame, WIDTH as i32 - 1, 0, self.progress_pct());
    }
}

fn aabb(ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}
