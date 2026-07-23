//! Thirty-four courses — HUD shows level number 1..=34.
//!
//! Terrain variety (difficulty still rises overall through 16):
//! - Flat: 1, 2, 9
//! - Smooth hills: 4, 6, 10, 14
//! - 8px steps: 3, 5, 7, 11, 12, 15
//! - Mixed jumps + gradients: 8, 13, 16
//! - Gravity flip + revert (2 flips): 17–21
//! - Multi gravity flips (3 / 4 / 5): 22–24
//! - Moving spikes: 25–26
//! - Speed-gate boxes (scroll 2↔4): 27–30
//! - Combined (gravity + movers + speed gates): 31–34

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Spike,
    /// Spike that slides horizontally around `x` over a short range.
    MovingSpike,
    Block,
    /// Looks like a block; clearing it (jumping over) toggles scroll speed.
    SpeedBlock,
}

#[derive(Clone, Copy)]
pub struct Obstacle {
    /// Anchor X (home for moving spikes).
    pub x: i32,
    pub kind: Kind,
}

/// Terrain control point: ground top-Y is `y` at world `x`.
#[derive(Clone, Copy)]
pub struct TerrainKey {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TerrainMode {
    /// Linear interpolation between keys (gentle hills).
    Smooth,
    /// Flat until each key, then instant height change (ledges).
    Stepped,
}

pub struct Level {
    pub difficulty: u8,
    pub length: i32,
    pub mode: TerrainMode,
    /// Number of gravity flip lines (0 = none). Odd crossings → inverted.
    pub gravity_flips: u8,
    pub terrain: &'static [TerrainKey],
    pub obstacles: &'static [Obstacle],
}

pub const LEVEL_COUNT: usize = 34;
pub const BASE_Y: i32 = 34;
/// Block / full-jump ledge height.
pub const STEP: i32 = 8;

/// Horizontal travel (±) for [`Kind::MovingSpike`].
pub const MOVE_SPIKE_AMP: i32 = 8;
/// Frames for a full back-and-forth cycle.
pub const MOVE_SPIKE_PERIOD: i32 = 48;

impl Obstacle {
    /// World X used for draw / collision this frame.
    pub fn world_x(&self, tick: u32) -> i32 {
        match self.kind {
            Kind::Spike | Kind::Block | Kind::SpeedBlock => self.x,
            Kind::MovingSpike => {
                let t = (tick as i32)
                    .wrapping_add(self.x.wrapping_mul(3))
                    .rem_euclid(MOVE_SPIKE_PERIOD);
                let half = MOVE_SPIKE_PERIOD / 2;
                let tri = if t <= half { t } else { MOVE_SPIKE_PERIOD - t };
                self.x - MOVE_SPIKE_AMP + (tri * 2 * MOVE_SPIKE_AMP) / half
            }
        }
    }
}

impl Level {
    /// World X of flip index `i` (0..gravity_flips), evenly spaced with end margins.
    pub fn flip_at(&self, i: u8) -> i32 {
        let n = self.gravity_flips.max(1) as i32;
        self.length * (2 * i as i32 + 1) / (2 * n)
    }

    pub fn near_flip(&self, world_x: i32) -> bool {
        for i in 0..self.gravity_flips {
            if (world_x - self.flip_at(i)).abs() <= 2 {
                return true;
            }
        }
        false
    }

    /// True when an odd number of flip lines have been crossed.
    pub fn inverted_at(&self, world_x: i32) -> bool {
        if self.gravity_flips == 0 {
            return false;
        }
        let mut passed = 0u8;
        for i in 0..self.gravity_flips {
            if world_x >= self.flip_at(i) {
                passed = passed.wrapping_add(1);
            }
        }
        passed % 2 == 1
    }
}

/// Ground Y at world x.
pub fn ground_at(level: &Level, x: i32) -> i32 {
    let keys = level.terrain;
    if keys.is_empty() {
        return BASE_Y;
    }
    match level.mode {
        TerrainMode::Stepped => {
            let mut y = keys[0].y;
            for k in keys {
                if x >= k.x {
                    y = k.y;
                } else {
                    break;
                }
            }
            y
        }
        TerrainMode::Smooth => {
            if x <= keys[0].x {
                return keys[0].y;
            }
            for i in 0..keys.len() - 1 {
                let a = keys[i];
                let b = keys[i + 1];
                if x <= b.x {
                    let dx = b.x - a.x;
                    if dx <= 0 {
                        return b.y;
                    }
                    let t = x - a.x;
                    return a.y + (b.y - a.y) * t / dx;
                }
            }
            keys[keys.len() - 1].y
        }
    }
}

pub fn get(index: u8) -> &'static Level {
    &LEVELS[index.min((LEVEL_COUNT - 1) as u8) as usize]
}

const Y0: i32 = BASE_Y;
const Y1: i32 = BASE_Y - STEP; // 26
const Y2: i32 = BASE_Y - STEP * 2; // 18

// --- Level 1: flat intro ---
const T1: &[TerrainKey] = &[TerrainKey { x: 0, y: Y0 }, TerrainKey { x: 560, y: Y0 }];
const O1: &[Obstacle] = &[
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 260, kind: Kind::Block },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Spike },
    Obstacle { x: 500, kind: Kind::Block },
];

// --- Level 2: flat denser ---
const T2: &[TerrainKey] = &[TerrainKey { x: 0, y: Y0 }, TerrainKey { x: 600, y: Y0 }];
const O2: &[Obstacle] = &[
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 150, kind: Kind::Spike },
    Obstacle { x: 220, kind: Kind::Block },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 400, kind: Kind::Spike },
    Obstacle { x: 480, kind: Kind::Block },
    Obstacle { x: 540, kind: Kind::Spike },
];

// --- Level 3: first 8px steps ---
const T3: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 160, y: Y1 },
    TerrainKey { x: 280, y: Y0 },
    TerrainKey { x: 400, y: Y1 },
    TerrainKey { x: 520, y: Y2 },
    TerrainKey { x: 640, y: Y1 },
    TerrainKey { x: 720, y: Y0 },
];
const O3: &[Obstacle] = &[
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::Spike },
    Obstacle { x: 200, kind: Kind::Block },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 310, kind: Kind::Spike },
    Obstacle { x: 380, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 470, kind: Kind::Spike },
    Obstacle { x: 560, kind: Kind::Block },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 660, kind: Kind::Spike },
];

// --- Level 4: gentle hills ---
const T4: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 60, y: 34 },
    TerrainKey { x: 140, y: 28 },
    TerrainKey { x: 220, y: 34 },
    TerrainKey { x: 320, y: 30 },
    TerrainKey { x: 400, y: 26 },
    TerrainKey { x: 500, y: 32 },
    TerrainKey { x: 600, y: 28 },
    TerrainKey { x: 720, y: 34 },
];
const O4: &[Obstacle] = &[
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 120, kind: Kind::Spike },
    Obstacle { x: 170, kind: Kind::Block },
    Obstacle { x: 210, kind: Kind::Spike },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 300, kind: Kind::Block },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Block },
    Obstacle { x: 455, kind: Kind::Spike },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Block },
    Obstacle { x: 545, kind: Kind::Spike },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 595, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::Block },
    Obstacle { x: 670, kind: Kind::Spike },
    Obstacle { x: 690, kind: Kind::Spike },
];

// --- Level 5: more 8px steps ---
const T5: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 120, y: Y1 },
    TerrainKey { x: 220, y: Y0 },
    TerrainKey { x: 320, y: Y1 },
    TerrainKey { x: 420, y: Y2 },
    TerrainKey { x: 540, y: Y1 },
    TerrainKey { x: 640, y: Y0 },
    TerrainKey { x: 760, y: Y0 },
];
const O5: &[Obstacle] = &[
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 110, kind: Kind::Spike },
    Obstacle { x: 150, kind: Kind::Block },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 230, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Block },
    Obstacle { x: 330, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 390, kind: Kind::Spike },
    Obstacle { x: 450, kind: Kind::Block },
    Obstacle { x: 500, kind: Kind::Spike },
    Obstacle { x: 530, kind: Kind::Spike },
    Obstacle { x: 580, kind: Kind::Block },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 650, kind: Kind::Spike },
    Obstacle { x: 690, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::Block },
];

// --- Level 6: steeper rolling hills ---
const T6: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 60, y: 26 },
    TerrainKey { x: 140, y: 34 },
    TerrainKey { x: 200, y: 22 },
    TerrainKey { x: 300, y: 30 },
    TerrainKey { x: 380, y: 24 },
    TerrainKey { x: 480, y: 34 },
    TerrainKey { x: 560, y: 26 },
    TerrainKey { x: 680, y: 32 },
    TerrainKey { x: 800, y: 34 },
];
const O6: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 210, kind: Kind::Spike },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 290, kind: Kind::Block },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 370, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Block },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Spike },
    Obstacle { x: 570, kind: Kind::Block },
    Obstacle { x: 610, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::Block },
    Obstacle { x: 760, kind: Kind::Spike },
];

// --- Level 7: dense 8px steps ---
const T7: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y1 },
    TerrainKey { x: 180, y: Y2 },
    TerrainKey { x: 280, y: Y1 },
    TerrainKey { x: 380, y: Y0 },
    TerrainKey { x: 480, y: Y1 },
    TerrainKey { x: 560, y: Y2 },
    TerrainKey { x: 660, y: Y1 },
    TerrainKey { x: 760, y: Y0 },
    TerrainKey { x: 840, y: Y0 },
];
const O7: &[Obstacle] = &[
    Obstacle { x: 60, kind: Kind::Spike },
    Obstacle { x: 95, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 175, kind: Kind::Spike },
    Obstacle { x: 205, kind: Kind::Spike },
    Obstacle { x: 235, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Block },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 350, kind: Kind::Spike },
    Obstacle { x: 390, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 535, kind: Kind::Block },
    Obstacle { x: 575, kind: Kind::Spike },
    Obstacle { x: 605, kind: Kind::Spike },
    Obstacle { x: 635, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Block },
    Obstacle { x: 720, kind: Kind::Spike },
    Obstacle { x: 750, kind: Kind::Spike },
    Obstacle { x: 780, kind: Kind::Spike },
    Obstacle { x: 810, kind: Kind::Block },
];

// --- Level 8: mixed 8px jumps + gradients ---
const T8: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 60, y: Y0 },
    TerrainKey { x: 62, y: Y1 },
    TerrainKey { x: 140, y: Y1 },
    TerrainKey { x: 200, y: 30 },
    TerrainKey { x: 280, y: 34 },
    TerrainKey { x: 340, y: Y0 },
    TerrainKey { x: 342, y: Y1 },
    TerrainKey { x: 400, y: Y1 },
    TerrainKey { x: 402, y: Y2 },
    TerrainKey { x: 480, y: Y2 },
    TerrainKey { x: 560, y: 26 },
    TerrainKey { x: 640, y: 32 },
    TerrainKey { x: 700, y: Y0 },
    TerrainKey { x: 702, y: Y1 },
    TerrainKey { x: 780, y: Y1 },
    TerrainKey { x: 820, y: 28 },
    TerrainKey { x: 900, y: Y0 },
];
const O8: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 110, kind: Kind::Block },
    Obstacle { x: 150, kind: Kind::Spike },
    Obstacle { x: 175, kind: Kind::Spike },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::Block },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 305, kind: Kind::Spike },
    Obstacle { x: 330, kind: Kind::Spike },
    Obstacle { x: 370, kind: Kind::Block },
    Obstacle { x: 410, kind: Kind::Spike },
    Obstacle { x: 435, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 500, kind: Kind::Block },
    Obstacle { x: 540, kind: Kind::Spike },
    Obstacle { x: 565, kind: Kind::Spike },
    Obstacle { x: 590, kind: Kind::Spike },
    Obstacle { x: 630, kind: Kind::Block },
    Obstacle { x: 670, kind: Kind::Spike },
    Obstacle { x: 695, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::Spike },
    Obstacle { x: 760, kind: Kind::Block },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 825, kind: Kind::Spike },
    Obstacle { x: 850, kind: Kind::Spike },
    Obstacle { x: 880, kind: Kind::Block },
];

// --- Level 9: flat gauntlet (variety reset, denser than 1–2) ---
const T9: &[TerrainKey] = &[TerrainKey { x: 0, y: Y0 }, TerrainKey { x: 920, y: Y0 }];
const O9: &[Obstacle] = &[
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::Block },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 210, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Block },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 350, kind: Kind::Spike },
    Obstacle { x: 390, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 455, kind: Kind::Spike },
    Obstacle { x: 480, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Block },
    Obstacle { x: 560, kind: Kind::Spike },
    Obstacle { x: 590, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 660, kind: Kind::Block },
    Obstacle { x: 700, kind: Kind::Spike },
    Obstacle { x: 730, kind: Kind::Spike },
    Obstacle { x: 770, kind: Kind::Block },
    Obstacle { x: 810, kind: Kind::Spike },
    Obstacle { x: 840, kind: Kind::Spike },
    Obstacle { x: 870, kind: Kind::Spike },
    Obstacle { x: 900, kind: Kind::Block },
];

// --- Level 10: long rolling waves ---
const T10: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 80, y: 28 },
    TerrainKey { x: 160, y: 34 },
    TerrainKey { x: 240, y: 24 },
    TerrainKey { x: 340, y: 32 },
    TerrainKey { x: 420, y: 22 },
    TerrainKey { x: 520, y: 30 },
    TerrainKey { x: 600, y: 26 },
    TerrainKey { x: 700, y: 34 },
    TerrainKey { x: 780, y: 24 },
    TerrainKey { x: 880, y: 30 },
    TerrainKey { x: 960, y: 34 },
];
const O10: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 170, kind: Kind::Spike },
    Obstacle { x: 210, kind: Kind::Spike },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 290, kind: Kind::Block },
    Obstacle { x: 330, kind: Kind::Spike },
    Obstacle { x: 370, kind: Kind::Spike },
    Obstacle { x: 410, kind: Kind::Spike },
    Obstacle { x: 450, kind: Kind::Block },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 530, kind: Kind::Spike },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 610, kind: Kind::Block },
    Obstacle { x: 650, kind: Kind::Spike },
    Obstacle { x: 690, kind: Kind::Spike },
    Obstacle { x: 730, kind: Kind::Spike },
    Obstacle { x: 770, kind: Kind::Block },
    Obstacle { x: 810, kind: Kind::Spike },
    Obstacle { x: 850, kind: Kind::Spike },
    Obstacle { x: 890, kind: Kind::Spike },
    Obstacle { x: 930, kind: Kind::Block },
];

// --- Level 11: step ladder (Y0→Y1→Y2 climbs) ---
const T11: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y1 },
    TerrainKey { x: 180, y: Y2 },
    TerrainKey { x: 280, y: Y1 },
    TerrainKey { x: 360, y: Y0 },
    TerrainKey { x: 440, y: Y1 },
    TerrainKey { x: 520, y: Y2 },
    TerrainKey { x: 620, y: Y1 },
    TerrainKey { x: 700, y: Y0 },
    TerrainKey { x: 780, y: Y1 },
    TerrainKey { x: 860, y: Y2 },
    TerrainKey { x: 960, y: Y1 },
    TerrainKey { x: 1040, y: Y0 },
];
const O11: &[Obstacle] = &[
    Obstacle { x: 55, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 165, kind: Kind::Spike },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::Block },
    Obstacle { x: 300, kind: Kind::Spike },
    Obstacle { x: 335, kind: Kind::Spike },
    Obstacle { x: 385, kind: Kind::Block },
    Obstacle { x: 420, kind: Kind::Spike },
    Obstacle { x: 455, kind: Kind::Spike },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 545, kind: Kind::Block },
    Obstacle { x: 585, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 660, kind: Kind::Block },
    Obstacle { x: 720, kind: Kind::Spike },
    Obstacle { x: 755, kind: Kind::Spike },
    Obstacle { x: 800, kind: Kind::Block },
    Obstacle { x: 840, kind: Kind::Spike },
    Obstacle { x: 875, kind: Kind::Spike },
    Obstacle { x: 910, kind: Kind::Spike },
    Obstacle { x: 960, kind: Kind::Block },
    Obstacle { x: 1000, kind: Kind::Spike },
];

// --- Level 12: wide plateaus, jump faces before spike packs ---
const T12: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 140, y: Y1 },
    TerrainKey { x: 300, y: Y1 },
    TerrainKey { x: 301, y: Y0 },
    TerrainKey { x: 420, y: Y0 },
    TerrainKey { x: 421, y: Y1 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 561, y: Y2 },
    TerrainKey { x: 720, y: Y2 },
    TerrainKey { x: 721, y: Y1 },
    TerrainKey { x: 860, y: Y1 },
    TerrainKey { x: 861, y: Y0 },
    TerrainKey { x: 1000, y: Y0 },
];
const O12: &[Obstacle] = &[
    Obstacle { x: 60, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 170, kind: Kind::Block },
    Obstacle { x: 220, kind: Kind::Spike },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 340, kind: Kind::Block },
    Obstacle { x: 380, kind: Kind::Spike },
    Obstacle { x: 410, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Block },
    Obstacle { x: 500, kind: Kind::Spike },
    Obstacle { x: 530, kind: Kind::Spike },
    Obstacle { x: 590, kind: Kind::Block },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 670, kind: Kind::Spike },
    Obstacle { x: 700, kind: Kind::Spike },
    Obstacle { x: 760, kind: Kind::Block },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 830, kind: Kind::Spike },
    Obstacle { x: 890, kind: Kind::Block },
    Obstacle { x: 930, kind: Kind::Spike },
    Obstacle { x: 960, kind: Kind::Spike },
];

// --- Level 13: mixed jumps + grads, tighter than 8 ---
const T13: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 50, y: Y0 },
    TerrainKey { x: 52, y: Y1 },
    TerrainKey { x: 110, y: Y1 },
    TerrainKey { x: 160, y: 30 },
    TerrainKey { x: 220, y: 34 },
    TerrainKey { x: 260, y: Y0 },
    TerrainKey { x: 262, y: Y1 },
    TerrainKey { x: 310, y: Y1 },
    TerrainKey { x: 312, y: Y2 },
    TerrainKey { x: 400, y: Y2 },
    TerrainKey { x: 460, y: 24 },
    TerrainKey { x: 540, y: 32 },
    TerrainKey { x: 580, y: Y0 },
    TerrainKey { x: 582, y: Y1 },
    TerrainKey { x: 640, y: Y1 },
    TerrainKey { x: 642, y: Y2 },
    TerrainKey { x: 720, y: Y2 },
    TerrainKey { x: 780, y: 28 },
    TerrainKey { x: 860, y: Y1 },
    TerrainKey { x: 862, y: Y0 },
    TerrainKey { x: 940, y: Y0 },
    TerrainKey { x: 942, y: Y1 },
    TerrainKey { x: 1020, y: Y1 },
    TerrainKey { x: 1080, y: Y0 },
];
const O13: &[Obstacle] = &[
    Obstacle { x: 30, kind: Kind::Spike },
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Block },
    Obstacle { x: 140, kind: Kind::Spike },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 230, kind: Kind::Block },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 330, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Block },
    Obstacle { x: 480, kind: Kind::Spike },
    Obstacle { x: 510, kind: Kind::Spike },
    Obstacle { x: 550, kind: Kind::Spike },
    Obstacle { x: 600, kind: Kind::Block },
    Obstacle { x: 660, kind: Kind::Spike },
    Obstacle { x: 690, kind: Kind::Spike },
    Obstacle { x: 740, kind: Kind::Block },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 830, kind: Kind::Spike },
    Obstacle { x: 880, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::Block },
    Obstacle { x: 970, kind: Kind::Spike },
    Obstacle { x: 1000, kind: Kind::Spike },
    Obstacle { x: 1040, kind: Kind::Block },
];

// --- Level 14: valley traps (deep dips + crest packs) ---
const T14: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 70, y: 26 },
    TerrainKey { x: 140, y: 34 },
    TerrainKey { x: 210, y: 20 },
    TerrainKey { x: 300, y: 32 },
    TerrainKey { x: 380, y: 22 },
    TerrainKey { x: 460, y: 34 },
    TerrainKey { x: 540, y: 24 },
    TerrainKey { x: 620, y: 18 },
    TerrainKey { x: 700, y: 30 },
    TerrainKey { x: 780, y: 22 },
    TerrainKey { x: 860, y: 34 },
    TerrainKey { x: 940, y: 26 },
    TerrainKey { x: 1020, y: 34 },
    TerrainKey { x: 1100, y: 34 },
];
const O14: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 120, kind: Kind::Block },
    Obstacle { x: 170, kind: Kind::Spike },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Block },
    Obstacle { x: 330, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 400, kind: Kind::Spike },
    Obstacle { x: 440, kind: Kind::Block },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Spike },
    Obstacle { x: 560, kind: Kind::Spike },
    Obstacle { x: 600, kind: Kind::Block },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 670, kind: Kind::Spike },
    Obstacle { x: 710, kind: Kind::Spike },
    Obstacle { x: 750, kind: Kind::Block },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 830, kind: Kind::Spike },
    Obstacle { x: 870, kind: Kind::Spike },
    Obstacle { x: 910, kind: Kind::Block },
    Obstacle { x: 960, kind: Kind::Spike },
    Obstacle { x: 990, kind: Kind::Spike },
    Obstacle { x: 1030, kind: Kind::Spike },
    Obstacle { x: 1060, kind: Kind::Block },
];

// --- Level 15: rapid 8px stairs, short runways ---
const T15: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 80, y: Y1 },
    TerrainKey { x: 140, y: Y2 },
    TerrainKey { x: 200, y: Y1 },
    TerrainKey { x: 260, y: Y0 },
    TerrainKey { x: 320, y: Y1 },
    TerrainKey { x: 380, y: Y2 },
    TerrainKey { x: 440, y: Y1 },
    TerrainKey { x: 500, y: Y0 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 620, y: Y2 },
    TerrainKey { x: 700, y: Y1 },
    TerrainKey { x: 760, y: Y0 },
    TerrainKey { x: 820, y: Y1 },
    TerrainKey { x: 880, y: Y2 },
    TerrainKey { x: 960, y: Y1 },
    TerrainKey { x: 1040, y: Y0 },
    TerrainKey { x: 1120, y: Y0 },
];
const O15: &[Obstacle] = &[
    Obstacle { x: 45, kind: Kind::Spike },
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 105, kind: Kind::Block },
    Obstacle { x: 155, kind: Kind::Spike },
    Obstacle { x: 185, kind: Kind::Spike },
    Obstacle { x: 225, kind: Kind::Block },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 310, kind: Kind::Spike },
    Obstacle { x: 350, kind: Kind::Block },
    Obstacle { x: 400, kind: Kind::Spike },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 470, kind: Kind::Block },
    Obstacle { x: 525, kind: Kind::Spike },
    Obstacle { x: 555, kind: Kind::Spike },
    Obstacle { x: 590, kind: Kind::Block },
    Obstacle { x: 645, kind: Kind::Spike },
    Obstacle { x: 675, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::Block },
    Obstacle { x: 780, kind: Kind::Spike },
    Obstacle { x: 810, kind: Kind::Spike },
    Obstacle { x: 850, kind: Kind::Block },
    Obstacle { x: 905, kind: Kind::Spike },
    Obstacle { x: 935, kind: Kind::Spike },
    Obstacle { x: 980, kind: Kind::Block },
    Obstacle { x: 1020, kind: Kind::Spike },
    Obstacle { x: 1050, kind: Kind::Spike },
    Obstacle { x: 1080, kind: Kind::Spike },
];

// --- Level 16: finale — flat → steps → hills → mixed jumps ---
const T16: &[TerrainKey] = &[
    // flat sprint
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 200, y: Y0 },
    // steps
    TerrainKey { x: 201, y: Y1 },
    TerrainKey { x: 280, y: Y1 },
    TerrainKey { x: 281, y: Y2 },
    TerrainKey { x: 360, y: Y2 },
    TerrainKey { x: 361, y: Y1 },
    TerrainKey { x: 440, y: Y1 },
    TerrainKey { x: 441, y: Y0 },
    // hills
    TerrainKey { x: 520, y: 28 },
    TerrainKey { x: 600, y: 22 },
    TerrainKey { x: 680, y: 32 },
    TerrainKey { x: 760, y: 24 },
    // mixed jumps
    TerrainKey { x: 820, y: Y0 },
    TerrainKey { x: 822, y: Y1 },
    TerrainKey { x: 900, y: Y1 },
    TerrainKey { x: 902, y: Y2 },
    TerrainKey { x: 980, y: Y2 },
    TerrainKey { x: 1040, y: 28 },
    TerrainKey { x: 1100, y: Y0 },
    TerrainKey { x: 1102, y: Y1 },
    TerrainKey { x: 1180, y: Y1 },
    TerrainKey { x: 1240, y: Y0 },
];
const O16: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 170, kind: Kind::Spike },
    Obstacle { x: 230, kind: Kind::Spike },
    Obstacle { x: 260, kind: Kind::Spike },
    Obstacle { x: 300, kind: Kind::Block },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 390, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Block },
    Obstacle { x: 480, kind: Kind::Spike },
    Obstacle { x: 540, kind: Kind::Spike },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::Block },
    Obstacle { x: 660, kind: Kind::Spike },
    Obstacle { x: 700, kind: Kind::Spike },
    Obstacle { x: 740, kind: Kind::Spike },
    Obstacle { x: 790, kind: Kind::Block },
    Obstacle { x: 840, kind: Kind::Spike },
    Obstacle { x: 870, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::Block },
    Obstacle { x: 960, kind: Kind::Spike },
    Obstacle { x: 1000, kind: Kind::Spike },
    Obstacle { x: 1060, kind: Kind::Block },
    Obstacle { x: 1120, kind: Kind::Spike },
    Obstacle { x: 1150, kind: Kind::Spike },
    Obstacle { x: 1190, kind: Kind::Spike },
    Obstacle { x: 1220, kind: Kind::Block },
];

// --- Level 17: gravity intro (diff 12) — stepped plateaus ---
// Flips at 240 / 720. Segments: normal → inverted → normal.
const T17: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 120, y: Y1 },
    TerrainKey { x: 240, y: Y1 },
    TerrainKey { x: 241, y: Y0 },
    TerrainKey { x: 400, y: Y0 },
    TerrainKey { x: 401, y: Y1 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 561, y: Y0 },
    TerrainKey { x: 720, y: Y0 },
    TerrainKey { x: 721, y: Y1 },
    TerrainKey { x: 860, y: Y1 },
    TerrainKey { x: 861, y: Y0 },
    TerrainKey { x: 960, y: Y0 },
];
const O17: &[Obstacle] = &[
    // normal 0–240
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Spike },
    Obstacle { x: 150, kind: Kind::Block },
    Obstacle { x: 200, kind: Kind::Spike },
    // inverted 240–720
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 370, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 480, kind: Kind::Spike },
    Obstacle { x: 530, kind: Kind::Block },
    Obstacle { x: 590, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Block },
    // normal 720–960
    Obstacle { x: 760, kind: Kind::Spike },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 850, kind: Kind::Block },
    Obstacle { x: 910, kind: Kind::Spike },
];

// --- Level 18: gravity hills (diff 13) — smooth ---
const T18: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 80, y: 28 },
    TerrainKey { x: 160, y: 34 },
    TerrainKey { x: 250, y: 34 },
    TerrainKey { x: 330, y: 26 },
    TerrainKey { x: 420, y: 32 },
    TerrainKey { x: 500, y: 24 },
    TerrainKey { x: 580, y: 30 },
    TerrainKey { x: 660, y: 34 },
    TerrainKey { x: 750, y: 34 },
    TerrainKey { x: 830, y: 28 },
    TerrainKey { x: 920, y: 34 },
    TerrainKey { x: 1000, y: 34 },
];
const O18: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::Block },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 290, kind: Kind::Spike },
    Obstacle { x: 350, kind: Kind::Block },
    Obstacle { x: 400, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Block },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Block },
    Obstacle { x: 790, kind: Kind::Spike },
    Obstacle { x: 850, kind: Kind::Spike },
    Obstacle { x: 900, kind: Kind::Block },
    Obstacle { x: 960, kind: Kind::Spike },
];

// --- Level 19: gravity steps (diff 14) ---
const T19: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y1 },
    TerrainKey { x: 200, y: Y0 },
    TerrainKey { x: 260, y: Y0 },
    TerrainKey { x: 340, y: Y1 },
    TerrainKey { x: 440, y: Y2 },
    TerrainKey { x: 540, y: Y1 },
    TerrainKey { x: 640, y: Y0 },
    TerrainKey { x: 720, y: Y1 },
    TerrainKey { x: 780, y: Y1 },
    TerrainKey { x: 860, y: Y0 },
    TerrainKey { x: 940, y: Y1 },
    TerrainKey { x: 1040, y: Y0 },
];
const O19: &[Obstacle] = &[
    Obstacle { x: 45, kind: Kind::Spike },
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 130, kind: Kind::Block },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 300, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Block },
    Obstacle { x: 410, kind: Kind::Spike },
    Obstacle { x: 470, kind: Kind::Spike },
    Obstacle { x: 510, kind: Kind::Block },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Block },
    Obstacle { x: 740, kind: Kind::Spike },
    Obstacle { x: 820, kind: Kind::Spike },
    Obstacle { x: 880, kind: Kind::Block },
    Obstacle { x: 930, kind: Kind::Spike },
    Obstacle { x: 980, kind: Kind::Spike },
];

// --- Level 20: gravity mixed (diff 15) — smooth jumps ---
const T20: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 60, y: Y0 },
    TerrainKey { x: 62, y: Y1 },
    TerrainKey { x: 140, y: Y1 },
    TerrainKey { x: 200, y: 30 },
    TerrainKey { x: 270, y: Y0 },
    TerrainKey { x: 272, y: Y1 },
    TerrainKey { x: 360, y: Y1 },
    TerrainKey { x: 362, y: Y2 },
    TerrainKey { x: 460, y: Y2 },
    TerrainKey { x: 530, y: 26 },
    TerrainKey { x: 620, y: Y1 },
    TerrainKey { x: 700, y: Y0 },
    TerrainKey { x: 810, y: Y0 },
    TerrainKey { x: 812, y: Y1 },
    TerrainKey { x: 900, y: Y1 },
    TerrainKey { x: 960, y: 28 },
    TerrainKey { x: 1080, y: Y0 },
];
const O20: &[Obstacle] = &[
    Obstacle { x: 30, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::Block },
    Obstacle { x: 160, kind: Kind::Spike },
    Obstacle { x: 220, kind: Kind::Spike },
    Obstacle { x: 310, kind: Kind::Spike },
    Obstacle { x: 380, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 490, kind: Kind::Spike },
    Obstacle { x: 560, kind: Kind::Block },
    Obstacle { x: 610, kind: Kind::Spike },
    Obstacle { x: 660, kind: Kind::Spike },
    Obstacle { x: 740, kind: Kind::Block },
    Obstacle { x: 850, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::Block },
    Obstacle { x: 990, kind: Kind::Spike },
    Obstacle { x: 1030, kind: Kind::Spike },
];

// --- Level 21: gravity finale (diff 16) ---
const T21: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y0 },
    TerrainKey { x: 101, y: Y1 },
    TerrainKey { x: 200, y: Y1 },
    TerrainKey { x: 280, y: Y1 },
    TerrainKey { x: 281, y: Y0 },
    TerrainKey { x: 380, y: Y0 },
    TerrainKey { x: 381, y: Y1 },
    TerrainKey { x: 480, y: Y1 },
    TerrainKey { x: 481, y: Y2 },
    TerrainKey { x: 600, y: Y2 },
    TerrainKey { x: 680, y: 26 },
    TerrainKey { x: 760, y: Y1 },
    TerrainKey { x: 840, y: Y1 },
    TerrainKey { x: 841, y: Y0 },
    TerrainKey { x: 940, y: Y0 },
    TerrainKey { x: 941, y: Y1 },
    TerrainKey { x: 1040, y: Y1 },
    TerrainKey { x: 1120, y: Y0 },
];
const O21: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::Block },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 360, kind: Kind::Block },
    Obstacle { x: 420, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::Block },
    Obstacle { x: 570, kind: Kind::Spike },
    Obstacle { x: 630, kind: Kind::Spike },
    Obstacle { x: 700, kind: Kind::Block },
    Obstacle { x: 750, kind: Kind::Spike },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 880, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::Block },
    Obstacle { x: 980, kind: Kind::Spike },
    Obstacle { x: 1020, kind: Kind::Spike },
    Obstacle { x: 1070, kind: Kind::Block },
];

// --- Level 22: like 14 valleys, 3 gravity flips ---
const T22: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 80, y: 26 },
    TerrainKey { x: 160, y: 34 },
    TerrainKey { x: 240, y: 22 },
    TerrainKey { x: 340, y: 32 },
    TerrainKey { x: 420, y: 24 },
    TerrainKey { x: 520, y: 34 },
    TerrainKey { x: 600, y: 26 },
    TerrainKey { x: 700, y: 18 },
    TerrainKey { x: 800, y: 30 },
    TerrainKey { x: 900, y: 22 },
    TerrainKey { x: 1000, y: 34 },
    TerrainKey { x: 1100, y: 34 },
];
const O22: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::Block },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::Block },
    Obstacle { x: 380, kind: Kind::Spike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 500, kind: Kind::Block },
    Obstacle { x: 560, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::Block },
    Obstacle { x: 740, kind: Kind::Spike },
    Obstacle { x: 820, kind: Kind::Spike },
    Obstacle { x: 860, kind: Kind::Block },
    Obstacle { x: 940, kind: Kind::Spike },
    Obstacle { x: 980, kind: Kind::Spike },
    Obstacle { x: 1040, kind: Kind::Block },
];

// --- Level 23: like 15 stairs, 4 gravity flips ---
const T23: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 90, y: Y1 },
    TerrainKey { x: 160, y: Y2 },
    TerrainKey { x: 240, y: Y1 },
    TerrainKey { x: 320, y: Y0 },
    TerrainKey { x: 400, y: Y1 },
    TerrainKey { x: 480, y: Y2 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 640, y: Y0 },
    TerrainKey { x: 720, y: Y1 },
    TerrainKey { x: 800, y: Y2 },
    TerrainKey { x: 900, y: Y1 },
    TerrainKey { x: 1000, y: Y0 },
    TerrainKey { x: 1100, y: Y1 },
    TerrainKey { x: 1200, y: Y0 },
];
const O23: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 120, kind: Kind::Block },
    Obstacle { x: 190, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 350, kind: Kind::Block },
    Obstacle { x: 430, kind: Kind::Spike },
    Obstacle { x: 510, kind: Kind::Spike },
    Obstacle { x: 590, kind: Kind::Block },
    Obstacle { x: 670, kind: Kind::Spike },
    Obstacle { x: 840, kind: Kind::Block },
    Obstacle { x: 940, kind: Kind::Spike },
    Obstacle { x: 1040, kind: Kind::Spike },
    Obstacle { x: 1120, kind: Kind::Block },
    Obstacle { x: 1160, kind: Kind::Spike },
];

// --- Level 24: like 16 mixed finale, 5 gravity flips ---
const T24: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 160, y: Y0 },
    TerrainKey { x: 161, y: Y1 },
    TerrainKey { x: 260, y: Y1 },
    TerrainKey { x: 261, y: Y2 },
    TerrainKey { x: 380, y: Y2 },
    TerrainKey { x: 381, y: Y1 },
    TerrainKey { x: 500, y: Y1 },
    TerrainKey { x: 501, y: Y0 },
    TerrainKey { x: 600, y: 28 },
    TerrainKey { x: 720, y: 22 },
    TerrainKey { x: 840, y: 32 },
    TerrainKey { x: 960, y: 24 },
    TerrainKey { x: 1080, y: Y0 },
    TerrainKey { x: 1081, y: Y1 },
    TerrainKey { x: 1180, y: Y1 },
    TerrainKey { x: 1181, y: Y2 },
    TerrainKey { x: 1280, y: Y2 },
    TerrainKey { x: 1360, y: Y0 },
];
const O24: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::Spike },
    Obstacle { x: 190, kind: Kind::Block },
    Obstacle { x: 230, kind: Kind::Spike },
    Obstacle { x: 300, kind: Kind::Spike },
    Obstacle { x: 420, kind: Kind::Block },
    Obstacle { x: 540, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 700, kind: Kind::Block },
    Obstacle { x: 780, kind: Kind::Spike },
    Obstacle { x: 880, kind: Kind::Spike },
    Obstacle { x: 1000, kind: Kind::Block },
    Obstacle { x: 1120, kind: Kind::Spike },
    Obstacle { x: 1220, kind: Kind::Spike },
    Obstacle { x: 1300, kind: Kind::Block },
    Obstacle { x: 1340, kind: Kind::Spike },
];

// --- Level 25: moving-spike intro (mix of static + sliding) ---
const T25: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 200, y: Y0 },
    TerrainKey { x: 280, y: 28 },
    TerrainKey { x: 400, y: Y0 },
    TerrainKey { x: 520, y: 26 },
    TerrainKey { x: 640, y: Y0 },
    TerrainKey { x: 760, y: 28 },
    TerrainKey { x: 880, y: Y0 },
    TerrainKey { x: 960, y: Y0 },
];
const O25: &[Obstacle] = &[
    Obstacle { x: 60, kind: Kind::Spike },
    Obstacle { x: 120, kind: Kind::MovingSpike },
    Obstacle { x: 180, kind: Kind::Block },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::MovingSpike },
    Obstacle { x: 380, kind: Kind::Spike },
    Obstacle { x: 440, kind: Kind::Block },
    Obstacle { x: 500, kind: Kind::MovingSpike },
    Obstacle { x: 560, kind: Kind::Spike },
    Obstacle { x: 620, kind: Kind::MovingSpike },
    Obstacle { x: 700, kind: Kind::Block },
    Obstacle { x: 740, kind: Kind::Spike },
    Obstacle { x: 800, kind: Kind::MovingSpike },
    Obstacle { x: 860, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::MovingSpike },
];

// --- Level 26: denser movers on stepped shelves ---
const T26: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 120, y: Y1 },
    TerrainKey { x: 280, y: Y0 },
    TerrainKey { x: 400, y: Y1 },
    TerrainKey { x: 520, y: Y2 },
    TerrainKey { x: 640, y: Y1 },
    TerrainKey { x: 760, y: Y0 },
    TerrainKey { x: 880, y: Y1 },
    TerrainKey { x: 1000, y: Y0 },
    TerrainKey { x: 1080, y: Y0 },
];
const O26: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::MovingSpike },
    Obstacle { x: 160, kind: Kind::Block },
    Obstacle { x: 200, kind: Kind::MovingSpike },
    Obstacle { x: 250, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::MovingSpike },
    Obstacle { x: 370, kind: Kind::Spike },
    Obstacle { x: 440, kind: Kind::MovingSpike },
    Obstacle { x: 480, kind: Kind::Block },
    Obstacle { x: 540, kind: Kind::MovingSpike },
    Obstacle { x: 580, kind: Kind::Spike },
    Obstacle { x: 650, kind: Kind::MovingSpike },
    Obstacle { x: 700, kind: Kind::Spike },
    Obstacle { x: 780, kind: Kind::MovingSpike },
    Obstacle { x: 820, kind: Kind::Block },
    Obstacle { x: 880, kind: Kind::MovingSpike },
    Obstacle { x: 940, kind: Kind::Spike },
    Obstacle { x: 1000, kind: Kind::MovingSpike },
    Obstacle { x: 1040, kind: Kind::Spike },
];

// --- Level 27: speed-gate intro (few SpeedBlocks) ---
const T27: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 960, y: Y0 },
];
const O27: &[Obstacle] = &[
    Obstacle { x: 80, kind: Kind::Spike },
    Obstacle { x: 160, kind: Kind::SpeedBlock },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 400, kind: Kind::SpeedBlock },
    Obstacle { x: 500, kind: Kind::Spike },
    Obstacle { x: 580, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::SpeedBlock },
    Obstacle { x: 780, kind: Kind::Spike },
    Obstacle { x: 860, kind: Kind::Spike },
    Obstacle { x: 920, kind: Kind::Block },
];

// --- Level 28: more gates + gentle hills ---
const T28: &[TerrainKey] = &[
    TerrainKey { x: 0, y: 34 },
    TerrainKey { x: 120, y: 28 },
    TerrainKey { x: 240, y: 34 },
    TerrainKey { x: 400, y: 26 },
    TerrainKey { x: 560, y: 34 },
    TerrainKey { x: 720, y: 28 },
    TerrainKey { x: 880, y: 34 },
    TerrainKey { x: 1040, y: 34 },
];
const O28: &[Obstacle] = &[
    Obstacle { x: 60, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::SpeedBlock },
    Obstacle { x: 200, kind: Kind::Spike },
    Obstacle { x: 280, kind: Kind::SpeedBlock },
    Obstacle { x: 360, kind: Kind::Spike },
    Obstacle { x: 440, kind: Kind::SpeedBlock },
    Obstacle { x: 520, kind: Kind::Spike },
    Obstacle { x: 600, kind: Kind::Spike },
    Obstacle { x: 680, kind: Kind::SpeedBlock },
    Obstacle { x: 760, kind: Kind::Spike },
    Obstacle { x: 840, kind: Kind::SpeedBlock },
    Obstacle { x: 920, kind: Kind::Spike },
    Obstacle { x: 980, kind: Kind::Block },
];

// --- Level 29: stepped + frequent speed gates ---
const T29: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 140, y: Y1 },
    TerrainKey { x: 300, y: Y0 },
    TerrainKey { x: 460, y: Y1 },
    TerrainKey { x: 620, y: Y2 },
    TerrainKey { x: 780, y: Y1 },
    TerrainKey { x: 940, y: Y0 },
    TerrainKey { x: 1100, y: Y0 },
];
const O29: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::SpeedBlock },
    Obstacle { x: 180, kind: Kind::Spike },
    Obstacle { x: 240, kind: Kind::SpeedBlock },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 400, kind: Kind::SpeedBlock },
    Obstacle { x: 500, kind: Kind::Spike },
    Obstacle { x: 560, kind: Kind::SpeedBlock },
    Obstacle { x: 660, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::SpeedBlock },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 860, kind: Kind::SpeedBlock },
    Obstacle { x: 940, kind: Kind::Spike },
    Obstacle { x: 1000, kind: Kind::SpeedBlock },
    Obstacle { x: 1060, kind: Kind::Spike },
];

// --- Level 30: densest speed-gate course ---
const T30: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y0 },
    TerrainKey { x: 101, y: Y1 },
    TerrainKey { x: 220, y: Y1 },
    TerrainKey { x: 300, y: 28 },
    TerrainKey { x: 420, y: Y0 },
    TerrainKey { x: 421, y: Y1 },
    TerrainKey { x: 540, y: Y1 },
    TerrainKey { x: 541, y: Y2 },
    TerrainKey { x: 680, y: Y2 },
    TerrainKey { x: 760, y: 26 },
    TerrainKey { x: 880, y: Y1 },
    TerrainKey { x: 1000, y: Y0 },
    TerrainKey { x: 1001, y: Y1 },
    TerrainKey { x: 1120, y: Y1 },
    TerrainKey { x: 1200, y: Y0 },
];
const O30: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 80, kind: Kind::SpeedBlock },
    Obstacle { x: 140, kind: Kind::Spike },
    Obstacle { x: 180, kind: Kind::SpeedBlock },
    Obstacle { x: 260, kind: Kind::SpeedBlock },
    Obstacle { x: 340, kind: Kind::Spike },
    Obstacle { x: 380, kind: Kind::SpeedBlock },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 500, kind: Kind::SpeedBlock },
    Obstacle { x: 580, kind: Kind::SpeedBlock },
    Obstacle { x: 640, kind: Kind::Spike },
    Obstacle { x: 720, kind: Kind::SpeedBlock },
    Obstacle { x: 800, kind: Kind::Spike },
    Obstacle { x: 840, kind: Kind::SpeedBlock },
    Obstacle { x: 920, kind: Kind::SpeedBlock },
    Obstacle { x: 980, kind: Kind::Spike },
    Obstacle { x: 1040, kind: Kind::SpeedBlock },
    Obstacle { x: 1100, kind: Kind::Spike },
    Obstacle { x: 1140, kind: Kind::SpeedBlock },
];

// --- Level 31: combo intro — 2 flips, light movers + speed gates ---
const T31: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 160, y: Y0 },
    TerrainKey { x: 280, y: 28 },
    TerrainKey { x: 400, y: Y0 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 720, y: Y0 },
    TerrainKey { x: 880, y: 28 },
    TerrainKey { x: 1040, y: Y0 },
    TerrainKey { x: 1120, y: Y0 },
];
const O31: &[Obstacle] = &[
    Obstacle { x: 70, kind: Kind::Spike },
    Obstacle { x: 140, kind: Kind::SpeedBlock },
    Obstacle { x: 220, kind: Kind::MovingSpike },
    Obstacle { x: 320, kind: Kind::Spike },
    Obstacle { x: 380, kind: Kind::SpeedBlock },
    Obstacle { x: 480, kind: Kind::MovingSpike },
    Obstacle { x: 540, kind: Kind::Spike },
    Obstacle { x: 640, kind: Kind::SpeedBlock },
    Obstacle { x: 760, kind: Kind::MovingSpike },
    Obstacle { x: 840, kind: Kind::Spike },
    Obstacle { x: 940, kind: Kind::SpeedBlock },
    Obstacle { x: 1020, kind: Kind::Spike },
    Obstacle { x: 1080, kind: Kind::MovingSpike },
];

// --- Level 32: longer, denser mix, still 2 flips ---
const T32: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 100, y: Y1 },
    TerrainKey { x: 240, y: Y0 },
    TerrainKey { x: 400, y: 28 },
    TerrainKey { x: 560, y: Y1 },
    TerrainKey { x: 720, y: Y2 },
    TerrainKey { x: 880, y: Y1 },
    TerrainKey { x: 1040, y: Y0 },
    TerrainKey { x: 1160, y: 28 },
    TerrainKey { x: 1240, y: Y0 },
];
const O32: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 110, kind: Kind::SpeedBlock },
    Obstacle { x: 180, kind: Kind::MovingSpike },
    Obstacle { x: 280, kind: Kind::Spike },
    Obstacle { x: 340, kind: Kind::SpeedBlock },
    Obstacle { x: 440, kind: Kind::MovingSpike },
    Obstacle { x: 500, kind: Kind::Spike },
    Obstacle { x: 580, kind: Kind::SpeedBlock },
    Obstacle { x: 660, kind: Kind::MovingSpike },
    Obstacle { x: 740, kind: Kind::Spike },
    Obstacle { x: 800, kind: Kind::SpeedBlock },
    Obstacle { x: 880, kind: Kind::MovingSpike },
    Obstacle { x: 960, kind: Kind::Spike },
    Obstacle { x: 1020, kind: Kind::SpeedBlock },
    Obstacle { x: 1100, kind: Kind::MovingSpike },
    Obstacle { x: 1180, kind: Kind::Spike },
    Obstacle { x: 1220, kind: Kind::SpeedBlock },
];

// --- Level 33: 3 flips, stepped shelves, more gates/movers ---
const T33: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 120, y: Y1 },
    TerrainKey { x: 260, y: Y0 },
    TerrainKey { x: 400, y: Y1 },
    TerrainKey { x: 540, y: Y2 },
    TerrainKey { x: 700, y: Y1 },
    TerrainKey { x: 860, y: Y0 },
    TerrainKey { x: 1000, y: Y1 },
    TerrainKey { x: 1140, y: Y2 },
    TerrainKey { x: 1280, y: Y1 },
    TerrainKey { x: 1360, y: Y0 },
];
const O33: &[Obstacle] = &[
    Obstacle { x: 40, kind: Kind::Spike },
    Obstacle { x: 90, kind: Kind::SpeedBlock },
    Obstacle { x: 160, kind: Kind::MovingSpike },
    Obstacle { x: 220, kind: Kind::Spike },
    Obstacle { x: 300, kind: Kind::SpeedBlock },
    Obstacle { x: 360, kind: Kind::MovingSpike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 520, kind: Kind::SpeedBlock },
    Obstacle { x: 600, kind: Kind::MovingSpike },
    Obstacle { x: 680, kind: Kind::Spike },
    Obstacle { x: 760, kind: Kind::SpeedBlock },
    Obstacle { x: 820, kind: Kind::MovingSpike },
    Obstacle { x: 920, kind: Kind::Spike },
    Obstacle { x: 980, kind: Kind::SpeedBlock },
    Obstacle { x: 1060, kind: Kind::MovingSpike },
    Obstacle { x: 1120, kind: Kind::Spike },
    Obstacle { x: 1200, kind: Kind::SpeedBlock },
    Obstacle { x: 1260, kind: Kind::MovingSpike },
    Obstacle { x: 1320, kind: Kind::Spike },
];

// --- Level 34: longest combo finale — 3 flips, packed hazards ---
const T34: &[TerrainKey] = &[
    TerrainKey { x: 0, y: Y0 },
    TerrainKey { x: 140, y: Y0 },
    TerrainKey { x: 141, y: Y1 },
    TerrainKey { x: 280, y: Y1 },
    TerrainKey { x: 360, y: 28 },
    TerrainKey { x: 500, y: Y0 },
    TerrainKey { x: 501, y: Y1 },
    TerrainKey { x: 640, y: Y1 },
    TerrainKey { x: 641, y: Y2 },
    TerrainKey { x: 800, y: Y2 },
    TerrainKey { x: 900, y: 26 },
    TerrainKey { x: 1040, y: Y1 },
    TerrainKey { x: 1180, y: Y0 },
    TerrainKey { x: 1181, y: Y1 },
    TerrainKey { x: 1320, y: Y1 },
    TerrainKey { x: 1321, y: Y2 },
    TerrainKey { x: 1440, y: Y1 },
    TerrainKey { x: 1500, y: Y0 },
];
const O34: &[Obstacle] = &[
    Obstacle { x: 50, kind: Kind::Spike },
    Obstacle { x: 100, kind: Kind::SpeedBlock },
    Obstacle { x: 180, kind: Kind::MovingSpike },
    Obstacle { x: 240, kind: Kind::Spike },
    Obstacle { x: 320, kind: Kind::SpeedBlock },
    Obstacle { x: 400, kind: Kind::MovingSpike },
    Obstacle { x: 460, kind: Kind::Spike },
    Obstacle { x: 540, kind: Kind::SpeedBlock },
    Obstacle { x: 600, kind: Kind::MovingSpike },
    Obstacle { x: 700, kind: Kind::Spike },
    Obstacle { x: 760, kind: Kind::SpeedBlock },
    Obstacle { x: 840, kind: Kind::MovingSpike },
    Obstacle { x: 920, kind: Kind::SpeedBlock },
    Obstacle { x: 1000, kind: Kind::Spike },
    Obstacle { x: 1080, kind: Kind::MovingSpike },
    Obstacle { x: 1140, kind: Kind::SpeedBlock },
    Obstacle { x: 1220, kind: Kind::MovingSpike },
    Obstacle { x: 1280, kind: Kind::Spike },
    Obstacle { x: 1360, kind: Kind::SpeedBlock },
    Obstacle { x: 1420, kind: Kind::MovingSpike },
    Obstacle { x: 1470, kind: Kind::Spike },
];

pub static LEVELS: [Level; LEVEL_COUNT] = [
    Level {
        difficulty: 1,
        length: 560,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T1,
        obstacles: O1,
    },
    Level {
        difficulty: 2,
        length: 600,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T2,
        obstacles: O2,
    },
    Level {
        difficulty: 3,
        length: 720,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T3,
        obstacles: O3,
    },
    Level {
        difficulty: 4,
        length: 720,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T4,
        obstacles: O4,
    },
    Level {
        difficulty: 5,
        length: 760,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T5,
        obstacles: O5,
    },
    Level {
        difficulty: 6,
        length: 800,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T6,
        obstacles: O6,
    },
    Level {
        difficulty: 7,
        length: 840,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T7,
        obstacles: O7,
    },
    Level {
        difficulty: 8,
        length: 900,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T8,
        obstacles: O8,
    },
    Level {
        difficulty: 9,
        length: 920,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T9,
        obstacles: O9,
    },
    Level {
        difficulty: 10,
        length: 960,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T10,
        obstacles: O10,
    },
    Level {
        difficulty: 11,
        length: 1040,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T11,
        obstacles: O11,
    },
    Level {
        difficulty: 12,
        length: 1000,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T12,
        obstacles: O12,
    },
    Level {
        difficulty: 13,
        length: 1080,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T13,
        obstacles: O13,
    },
    Level {
        difficulty: 14,
        length: 1100,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T14,
        obstacles: O14,
    },
    Level {
        difficulty: 15,
        length: 1120,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T15,
        obstacles: O15,
    },
    Level {
        difficulty: 16,
        length: 1240,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T16,
        obstacles: O16,
    },
    Level {
        difficulty: 17,
        length: 960,
        mode: TerrainMode::Stepped,
        gravity_flips: 2,
        terrain: T17,
        obstacles: O17,
    },
    Level {
        difficulty: 18,
        length: 1000,
        mode: TerrainMode::Smooth,
        gravity_flips: 2,
        terrain: T18,
        obstacles: O18,
    },
    Level {
        difficulty: 19,
        length: 1040,
        mode: TerrainMode::Stepped,
        gravity_flips: 2,
        terrain: T19,
        obstacles: O19,
    },
    Level {
        difficulty: 20,
        length: 1080,
        mode: TerrainMode::Smooth,
        gravity_flips: 2,
        terrain: T20,
        obstacles: O20,
    },
    Level {
        difficulty: 21,
        length: 1120,
        mode: TerrainMode::Stepped,
        gravity_flips: 2,
        terrain: T21,
        obstacles: O21,
    },
    Level {
        difficulty: 22,
        length: 1100,
        mode: TerrainMode::Smooth,
        gravity_flips: 3,
        terrain: T22,
        obstacles: O22,
    },
    Level {
        difficulty: 23,
        length: 1200,
        mode: TerrainMode::Stepped,
        gravity_flips: 4,
        terrain: T23,
        obstacles: O23,
    },
    Level {
        difficulty: 24,
        length: 1360,
        mode: TerrainMode::Smooth,
        gravity_flips: 5,
        terrain: T24,
        obstacles: O24,
    },
    Level {
        difficulty: 25,
        length: 960,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T25,
        obstacles: O25,
    },
    Level {
        difficulty: 26,
        length: 1080,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T26,
        obstacles: O26,
    },
    Level {
        difficulty: 27,
        length: 960,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T27,
        obstacles: O27,
    },
    Level {
        difficulty: 28,
        length: 1040,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T28,
        obstacles: O28,
    },
    Level {
        difficulty: 29,
        length: 1100,
        mode: TerrainMode::Stepped,
        gravity_flips: 0,
        terrain: T29,
        obstacles: O29,
    },
    Level {
        difficulty: 30,
        length: 1200,
        mode: TerrainMode::Smooth,
        gravity_flips: 0,
        terrain: T30,
        obstacles: O30,
    },
    Level {
        difficulty: 31,
        length: 1120,
        mode: TerrainMode::Smooth,
        gravity_flips: 2,
        terrain: T31,
        obstacles: O31,
    },
    Level {
        difficulty: 32,
        length: 1240,
        mode: TerrainMode::Stepped,
        gravity_flips: 2,
        terrain: T32,
        obstacles: O32,
    },
    Level {
        difficulty: 33,
        length: 1360,
        mode: TerrainMode::Stepped,
        gravity_flips: 3,
        terrain: T33,
        obstacles: O33,
    },
    Level {
        difficulty: 34,
        length: 1500,
        mode: TerrainMode::Smooth,
        gravity_flips: 3,
        terrain: T34,
        obstacles: O34,
    },
];
