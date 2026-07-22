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
const SCROLL_SLOW: i32 = 2;
const SCROLL_FAST: i32 = 4;

const SPIKE_W: i32 = 6;
const SPIKE_H: i32 = 6;
const BLOCK_W: i32 = 6;
const BLOCK_H: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Playing,
    Dead { timer: u8 },
    Won { timer: u8 },
    /// Finished all levels — complete splash; BOOT tap ignored (wipe via splash hold).
    Complete { timer: u8 },
}

pub struct Game {
    level_index: u8,
    scroll: i32,
    /// Current scroll step (2 or 4); toggled by clearing SpeedBlocks.
    scroll_speed: i32,
    /// Bit i set ⇒ SpeedBlock at obstacles[i] already triggered.
    speed_triggers: u32,
    y: i32,
    vy: i32,
    on_ground: bool,
    /// Animation clock for moving hazards (advances while playing).
    tick: u32,
    phase: Phase,
    attempts: u32,
}

impl Game {
    pub fn new(saved_level: u8) -> Self {
        let level_index = saved_level.min((LEVEL_COUNT - 1) as u8);
        let mut g = Self {
            level_index,
            scroll: 0,
            scroll_speed: SCROLL_SLOW,
            speed_triggers: 0,
            y: 0,
            vy: 0,
            on_ground: true,
            tick: 0,
            phase: Phase::Playing,
            attempts: 1,
        };
        g.reset_run();
        g
    }

    /// Boot into the all-clear splash (saved progress is [`save::ALL_CLEAR`]).
    pub fn all_clear() -> Self {
        Self {
            level_index: (LEVEL_COUNT - 1) as u8,
            scroll: 0,
            scroll_speed: SCROLL_SLOW,
            speed_triggers: 0,
            y: 0,
            vy: 0,
            on_ground: true,
            tick: 0,
            phase: Phase::Complete { timer: 0 },
            attempts: 1,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete { .. })
    }

    /// After wipe from the complete screen — back to level 1.
    pub fn restart_from_wipe(&mut self) {
        self.level_index = 0;
        self.attempts = 1;
        self.reset_run();
    }

    fn level(&self) -> &'static Level {
        level::get(self.level_index)
    }

    fn player_world_x(&self) -> i32 {
        self.scroll + PLAYER_SX
    }

    /// Gravity / surface inverted after an odd number of flip lines.
    fn inverted_at(&self, world_x: i32) -> bool {
        self.level().inverted_at(world_x)
    }

    fn inverted(&self) -> bool {
        self.inverted_at(self.player_world_x() + CUBE / 2)
    }

    /// Walkable surface Y at world x (floor top normally; ceiling when inverted).
    fn surface_at(&self, world_x: i32) -> i32 {
        let g = ground_at(self.level(), world_x);
        if self.inverted_at(world_x) {
            (HEIGHT as i32 - 1) - g
        } else {
            g
        }
    }

    /// Cube Y when resting on `surface` (feet on floor, or top on ceiling).
    fn rest_y(surface: i32, inverted: bool) -> i32 {
        if inverted {
            surface
        } else {
            surface - CUBE
        }
    }

    /// Contact edge toward the active surface (feet normally, top when inverted).
    fn contact_y(&self) -> i32 {
        if self.inverted() {
            self.y
        } else {
            self.y + CUBE
        }
    }

    /// Dirt / ceiling under the cube, or a block platform if we're on / landing on it.
    fn floor_at_player(&self) -> i32 {
        let lvl = self.level();
        let px = self.player_world_x();
        let inv = self.inverted();
        let mut floor = self.surface_at(px + CUBE / 2);
        let contact = self.contact_y();

        for obs in lvl.obstacles {
            if obs.kind != Kind::Block && obs.kind != Kind::SpeedBlock {
                continue;
            }
            let ox = obs.x;
            if px + CUBE <= ox || px >= ox + BLOCK_W {
                continue;
            }
            let mid = ox + BLOCK_W / 2;
            let surf = self.surface_at(mid);
            let platform = if self.inverted_at(mid) {
                surf + BLOCK_H
            } else {
                surf - BLOCK_H
            };
            // Airborne: block is a platform. Grounded: only if already on top.
            let on_or_landing = if inv {
                !self.on_ground || contact + 3 >= platform
            } else {
                !self.on_ground || contact <= platform + 3
            };
            if on_or_landing {
                if inv {
                    if platform > floor {
                        floor = platform;
                    }
                } else if platform < floor {
                    floor = platform;
                }
            }
        }
        floor
    }

    fn reset_run(&mut self) {
        self.scroll = 0;
        self.scroll_speed = SCROLL_SLOW;
        self.speed_triggers = 0;
        self.vy = 0;
        self.tick = 0;
        self.on_ground = true;
        self.phase = Phase::Playing;
        let inv = self.inverted();
        self.y = Self::rest_y(self.floor_at_player(), inv);
    }

    fn progress_pct(&self) -> u32 {
        let len = self.level().length.max(1);
        ((self.scroll * 100) / len).min(100) as u32
    }

    /// Returns `true` on the frame a level is cleared (for LED celebrate).
    pub fn update(&mut self, jump_pressed: bool) -> bool {
        match self.phase {
            Phase::Dead { timer } => {
                if timer > 0 {
                    self.phase = Phase::Dead { timer: timer - 1 };
                } else if jump_pressed {
                    self.attempts = self.attempts.saturating_add(1);
                    self.reset_run();
                }
                return false;
            }
            Phase::Won { timer } => {
                if timer > 0 {
                    self.phase = Phase::Won { timer: timer - 1 };
                } else if jump_pressed {
                    self.advance_after_win();
                }
                return false;
            }
            Phase::Complete { timer } => {
                if timer > 0 {
                    self.phase = Phase::Complete { timer: timer - 1 };
                }
                // Stay on the complete splash; BOOT tap does nothing.
                // Reset is via power-cycle splash hold (wipe), same as usual.
                let _ = jump_pressed;
                return false;
            }
            Phase::Playing => {}
        }

        let prev_inv = self.inverted();
        let prev_px = self.player_world_x();

        // Scroll first so a jump on the frame a block arrives can still clear it.
        self.scroll += self.scroll_speed;
        self.tick = self.tick.wrapping_add(1);

        if self.scroll >= self.level().length {
            self.on_level_complete();
            return true;
        }

        self.check_speed_gates(prev_px);

        // Crossing a flip line: drop grounded state so we don't stick in the old surface.
        if self.inverted() != prev_inv {
            let was_grounded = self.on_ground;
            self.on_ground = false;
            if was_grounded {
                let inv = self.inverted();
                self.y = Self::rest_y(self.floor_at_player(), inv);
                self.vy = 0;
                self.on_ground = true;
            }
        }

        if jump_pressed && self.on_ground {
            self.vy = if self.inverted() { -JUMP_VEL } else { JUMP_VEL };
            self.on_ground = false;
        }

        // Only integrate vertical motion while airborne. Grounded frames would
        // otherwise sink 1px into the dirt every tick.
        if !self.on_ground {
            if self.inverted() {
                self.vy -= GRAVITY;
                if self.vy < -4 {
                    self.vy = -4;
                }
            } else {
                self.vy += GRAVITY;
                if self.vy > 4 {
                    self.vy = 4;
                }
            }
            self.y += self.vy;
        } else {
            self.vy = 0;
        }

        self.resolve_terrain();

        if self.hits_hazard() || self.buried_in_terrain() {
            self.phase = Phase::Dead { timer: 40 };
        }
        false
    }

    fn on_level_complete(&mut self) {
        // RAM only — main defers flash flush (immediate write hangs this board).
        if (self.level_index as usize) + 1 < LEVEL_COUNT {
            save::set_level_ram(self.level_index + 1);
            self.phase = Phase::Won { timer: 70 };
        } else {
            save::set_all_clear_ram();
            self.phase = Phase::Complete { timer: 90 };
        }
    }

    /// After the cube clears the right edge of a SpeedBlock, toggle scroll 2↔4.
    /// Surviving past the block implies a jump-over (walking into the face is lethal).
    fn check_speed_gates(&mut self, prev_px: i32) {
        let px = self.player_world_x();
        let lvl = self.level();
        for (i, obs) in lvl.obstacles.iter().enumerate() {
            if obs.kind != Kind::SpeedBlock {
                continue;
            }
            if i >= 32 {
                break;
            }
            let bit = 1u32 << i;
            if self.speed_triggers & bit != 0 {
                continue;
            }
            let right = obs.x + BLOCK_W;
            // Crossed the gate's trailing edge this frame.
            if prev_px <= right && px > right {
                self.scroll_speed = if self.scroll_speed == SCROLL_SLOW {
                    SCROLL_FAST
                } else {
                    SCROLL_SLOW
                };
                self.speed_triggers |= bit;
            }
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
        let inv = self.inverted();
        let contact = self.contact_y();

        if !self.on_ground {
            if inv {
                // Jumping into a hanging block underside: catch onto it.
                if contact < g && self.player_over_block_platform(g) {
                    self.y = Self::rest_y(g, true);
                    self.vy = 0;
                    self.on_ground = true;
                    return;
                }
                if self.vy <= 0 && contact <= g {
                    self.y = Self::rest_y(g, true);
                    self.vy = 0;
                    self.on_ground = true;
                }
            } else {
                // Jumping into a block face: catch onto the top instead of dying.
                if contact > g && self.player_over_block_platform(g) {
                    self.y = Self::rest_y(g, false);
                    self.vy = 0;
                    self.on_ground = true;
                    return;
                }
                if self.vy >= 0 && contact >= g {
                    self.y = Self::rest_y(g, false);
                    self.vy = 0;
                    self.on_ground = true;
                }
            }
            return;
        }

        if inv {
            let sink = g - contact; // positive if surface moved away into playfield
            if self.level().mode == TerrainMode::Stepped {
                if g < contact {
                    self.on_ground = false;
                } else if sink >= 0 {
                    self.y = Self::rest_y(g, true);
                }
            } else {
                let s_prev = self.surface_at(
                    self.scroll.saturating_sub(3) + PLAYER_SX + CUBE / 2,
                );
                // Positive when ceiling rises toward gravity (smaller y).
                let slope_up = s_prev - g;
                if slope_up <= 2 && sink <= 3 {
                    self.y = Self::rest_y(g, true);
                } else if g < contact {
                    self.on_ground = false;
                }
            }
        } else {
            let rise = contact - g;
            if self.level().mode == TerrainMode::Stepped {
                if g > contact {
                    self.on_ground = false;
                } else if rise <= 0 {
                    self.y = Self::rest_y(g, false);
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
                    self.y = Self::rest_y(g, false);
                } else if g > contact {
                    self.on_ground = false;
                }
            }
        }
    }

    /// True when `platform_y` is a block top/underside under the player (not plain dirt).
    fn player_over_block_platform(&self, platform_y: i32) -> bool {
        let lvl = self.level();
        let px = self.player_world_x();
        for obs in lvl.obstacles {
            if obs.kind != Kind::Block && obs.kind != Kind::SpeedBlock {
                continue;
            }
            let ox = obs.x;
            if px + CUBE <= ox || px >= ox + BLOCK_W {
                continue;
            }
            let mid = ox + BLOCK_W / 2;
            let surf = self.surface_at(mid);
            let top = if self.inverted_at(mid) {
                surf + BLOCK_H
            } else {
                surf - BLOCK_H
            };
            if top == platform_y {
                return true;
            }
        }
        false
    }

    fn buried_in_terrain(&self) -> bool {
        let inv = self.inverted();
        let dirt = self.surface_at(self.player_world_x() + CUBE / 2);
        let contact = self.contact_y();

        if inv {
            let sink = dirt - contact;
            if self.on_ground {
                // Standing on a hanging block: contact well below ceiling dirt.
                if contact > dirt + 2 {
                    return false;
                }
                return match self.level().mode {
                    TerrainMode::Stepped => sink < 0,
                    TerrainMode::Smooth => sink < -3,
                };
            }
            // Airborne: buried if deep into ceiling solid.
            sink < -4 && self.y < dirt
        } else {
            let rise = contact - dirt;
            if self.on_ground {
                // Standing on a block: feet well above dirt — not buried.
                if contact + 2 < dirt {
                    return false;
                }
                return match self.level().mode {
                    TerrainMode::Stepped => rise > 0,
                    TerrainMode::Smooth => rise > 3,
                };
            }
            rise > 4 && self.y < dirt
        }
    }

    fn hits_hazard(&self) -> bool {
        let px = self.player_world_x();
        let py = self.y;
        let contact = self.contact_y();
        let lvl = self.level();
        for obs in lvl.obstacles {
            match obs.kind {
                Kind::Spike | Kind::MovingSpike => {
                    let sx = obs.world_x(self.tick);
                    let mid = sx + SPIKE_W / 2;
                    let surf = self.surface_at(mid);
                    let (sy, sh) = if self.inverted_at(mid) {
                        (surf, SPIKE_H)
                    } else {
                        (surf - SPIKE_H, SPIKE_H)
                    };
                    if aabb(px, py, CUBE, CUBE, sx + 1, sy + 1, SPIKE_W - 2, sh - 1) {
                        return true;
                    }
                }
                Kind::Block | Kind::SpeedBlock => {
                    let mid = obs.x + BLOCK_W / 2;
                    let surf = self.surface_at(mid);
                    let bx = obs.x;
                    let (top, h) = if self.inverted_at(mid) {
                        (surf, BLOCK_H)
                    } else {
                        (surf - BLOCK_H, BLOCK_H)
                    };
                    if !aabb(px, py, CUBE, CUBE, bx, top, BLOCK_W, h) {
                        continue;
                    }
                    // Walkable face is a platform — only the body / far face is lethal.
                    if self.inverted_at(mid) {
                        let platform = surf + BLOCK_H;
                        if contact + 3 >= platform {
                            continue;
                        }
                    } else if contact <= top + 3 {
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
        let fill = self.level_index as usize % 6;

        // Variable-height ground / ceiling fill. Style cycles every 6 levels.
        let mut s_prev = self.surface_at(self.scroll);
        let mut inv_prev = self.inverted_at(self.scroll);
        for sx in 0..WIDTH as i32 {
            let wx = self.scroll + sx;
            let inv = self.inverted_at(wx);
            let s = self.surface_at(wx);
            let near_flip = lvl.near_flip(wx);

            if s >= 0 && s < HEIGHT as i32 {
                // Contour line so black-fill levels still show the terrain edge.
                framebuf::set_pixel(frame, sx, s, true);
            }
            // Vertical faces for real ledges only — never at a gravity seam
            // (including terrain steps right next to the flip, which read as a
            // solid bar into the new ceiling outline).
            if sx > 0 && s != s_prev && inv == inv_prev && !near_flip {
                let dy = (s - s_prev).abs();
                if dy <= level::STEP * 2 {
                    let y0 = s.min(s_prev);
                    let y1 = s.max(s_prev);
                    for y in y0..=y1 {
                        framebuf::set_pixel(frame, sx, y, true);
                    }
                }
            }
            // No fill on the gravity seam — ceiling fill's leading column would
            // otherwise look like a solid vertical bar.
            if !near_flip && !(sx > 0 && inv != inv_prev) {
                if inv {
                    for y in 0..s {
                        let on = match fill {
                            0 => ((sx + y + self.scroll) & 2) == 0,
                            1 => ((sx - y + self.scroll) & 2) == 0,
                            2 => true,
                            3 => (sx.wrapping_add(self.scroll) & 1) == 0 && (y & 1) == 0,
                            4 => false,
                            _ => !((sx.wrapping_add(self.scroll) & 1) == 0 && (y & 1) == 0),
                        };
                        if on {
                            framebuf::set_pixel(frame, sx, y, true);
                        }
                    }
                } else {
                    for y in (s + 1)..(HEIGHT as i32) {
                        let on = match fill {
                            0 => ((sx + y + self.scroll) & 2) == 0,
                            1 => ((sx - y + self.scroll) & 2) == 0,
                            2 => true,
                            3 => (sx.wrapping_add(self.scroll) & 1) == 0 && (y & 1) == 0,
                            4 => false,
                            _ => !((sx.wrapping_add(self.scroll) & 1) == 0 && (y & 1) == 0),
                        };
                        if on {
                            framebuf::set_pixel(frame, sx, y, true);
                        }
                    }
                }
            }
            s_prev = s;
            inv_prev = inv;
        }

        for obs in lvl.obstacles {
            let ox = obs.world_x(self.tick);
            let sx = ox - self.scroll;
            if sx < -10 || sx > WIDTH as i32 + 2 {
                continue;
            }
            let mid = ox
                + match obs.kind {
                    Kind::Spike | Kind::MovingSpike => SPIKE_W / 2,
                    Kind::Block | Kind::SpeedBlock => BLOCK_W / 2,
                };
            let surf = self.surface_at(mid);
            let inv = self.inverted_at(mid);
            match obs.kind {
                Kind::Spike => {
                    if inv {
                        framebuf::fill_spike_down(frame, sx, surf, SPIKE_W, SPIKE_H);
                    } else {
                        framebuf::fill_spike_up(frame, sx, surf - 1, SPIKE_W, SPIKE_H);
                    }
                }
                Kind::MovingSpike => {
                    if inv {
                        framebuf::stroke_spike_down(frame, sx, surf, SPIKE_W, SPIKE_H);
                    } else {
                        framebuf::stroke_spike_up(frame, sx, surf - 1, SPIKE_W, SPIKE_H);
                    }
                }
                Kind::Block => {
                    if inv {
                        framebuf::fill_rect(frame, sx, surf, BLOCK_W, BLOCK_H);
                    } else {
                        framebuf::fill_rect(frame, sx, surf - BLOCK_H, BLOCK_W, BLOCK_H);
                    }
                }
                Kind::SpeedBlock => {
                    // Hollow rect so speed gates read differently from solid blocks.
                    if inv {
                        framebuf::stroke_rect(frame, sx, surf, BLOCK_W, BLOCK_H);
                    } else {
                        framebuf::stroke_rect(frame, sx, surf - BLOCK_H, BLOCK_W, BLOCK_H);
                    }
                }
            }
        }

        let show_cube = match self.phase {
            Phase::Dead { timer } => (timer & 2) == 0,
            _ => true,
        };
        if show_cube {
            // Outline on black-fill levels so the cube stays visible; otherwise solid.
            let ground_fill = self.level_index as usize % 6;
            if ground_fill == 4 {
                framebuf::stroke_rect(frame, PLAYER_SX, self.y, CUBE, CUBE);
            } else {
                framebuf::fill_rect(frame, PLAYER_SX, self.y, CUBE, CUBE);
            }
        }

        // Gravity flip markers: full-height dotted line (1 on, 3 off), XOR so it
        // stays visible on any fill. Solid seam bars are avoided by skipping fill
        // / vertical faces near the flip — not by omitting this marker.
        if lvl.gravity_flips > 0 {
            for i in 0..lvl.gravity_flips {
                let sx = lvl.flip_at(i) - self.scroll;
                if sx < 0 || sx >= WIDTH as i32 {
                    continue;
                }
                let mut y = 0;
                while y < HEIGHT as i32 {
                    if y % 4 == 0 {
                        framebuf::xor_pixel(frame, sx, y);
                    }
                    y += 1;
                }
            }
        }

        // HUD: level number left, progress % right
        framebuf::draw_score(frame, 11, 0, u32::from(lvl.difficulty));
        framebuf::draw_score(frame, WIDTH as i32 - 1, 0, self.progress_pct());
    }
}

fn aabb(ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}
