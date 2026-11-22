//! Pure, engine-free menu rules (unit-tested): item order and availability,
//! keyboard navigation, activation, animation frame timing and the blink
//! schedule.

/// Main-menu entries, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    SinglePlayer,
    Multiplayer,
    Settings,
}

impl MenuItem {
    pub const ALL: [MenuItem; 3] = [
        MenuItem::SinglePlayer,
        MenuItem::Multiplayer,
        MenuItem::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MenuItem::SinglePlayer => "Tek Oyunculu",
            MenuItem::Multiplayer => "Çok Oyunculu",
            MenuItem::Settings => "Ayarlar",
        }
    }

    /// Only single player is wired up for now. Enabling another entry later
    /// means flipping it here and handling its [`MenuAction`].
    pub fn enabled(self) -> bool {
        matches!(self, MenuItem::SinglePlayer)
    }

    /// Why a disabled entry cannot be used (shown under the buttons).
    pub fn disabled_hint(self) -> Option<&'static str> {
        match self {
            MenuItem::SinglePlayer => None,
            MenuItem::Multiplayer => Some("Çok oyunculu mod yakında geliyor"),
            MenuItem::Settings => Some("Ayarlar henüz hazır değil"),
        }
    }
}

/// What activating an entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    StartSinglePlayer,
}

/// Menu selection state (a Bevy resource; the rules stay engine-free).
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuModel {
    pub selected: usize,
}

impl MenuModel {
    pub fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }

    /// Move the highlight by `delta` (wraps). Disabled entries can be
    /// highlighted (so their "coming soon" hint is readable) but never run.
    pub fn navigate(&mut self, delta: i32) {
        let n = MenuItem::ALL.len() as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
    }

    pub fn select(&mut self, index: usize) {
        if index < MenuItem::ALL.len() {
            self.selected = index;
        }
    }

    /// Activate entry `index`; `None` for disabled entries.
    pub fn activate(&self, index: usize) -> Option<MenuAction> {
        match MenuItem::ALL.get(index)? {
            MenuItem::SinglePlayer => Some(MenuAction::StartSinglePlayer),
            MenuItem::Multiplayer | MenuItem::Settings => None,
        }
    }

    pub fn activate_selected(&self) -> Option<MenuAction> {
        self.activate(self.selected)
    }
}

/// Frame of a looping clip from wall-clock seconds: independent of the
/// render frame rate.
pub fn loop_frame(elapsed: f64, fps: f32, frames: usize) -> usize {
    if frames == 0 {
        return 0;
    }
    ((elapsed * fps as f64).floor() as u64 % frames as u64) as usize
}

/// Blink phases in seconds: half-closed, closed, half-closed.
pub const BLINK_HALF: f32 = 0.05;
pub const BLINK_CLOSED: f32 = 0.09;
pub const BLINK_TOTAL: f32 = BLINK_HALF * 2.0 + BLINK_CLOSED;

fn hash01(seed: u32, k: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Irregular, deterministic blink timing for one character.
#[derive(Debug, Clone)]
pub struct Blinker {
    seed: u32,
    k: u32,
    /// Start time of the current/next blink.
    start: f32,
    /// Base gap between blinks (seconds); jittered per blink.
    min_gap: f32,
    jitter: f32,
}

impl Blinker {
    pub fn new(seed: u32, first_at: f32, min_gap: f32, jitter: f32) -> Self {
        Self {
            seed,
            k: 0,
            start: first_at,
            min_gap,
            jitter,
        }
    }

    /// Eyelid frame at time `now` (non-decreasing): `None` open, `Some(0)`
    /// half-closed, `Some(1)` closed.
    pub fn frame(&mut self, now: f32) -> Option<usize> {
        while now >= self.start + BLINK_TOTAL {
            self.k += 1;
            let r = hash01(self.seed, self.k);
            // One blink in six is a quick double blink.
            let gap = if hash01(self.seed ^ 0xA5A5, self.k) < 0.16 {
                0.22
            } else {
                self.min_gap + self.jitter * r
            };
            self.start += BLINK_TOTAL + gap;
        }
        let t = now - self.start;
        if t < 0.0 {
            None
        } else if t < BLINK_HALF || t >= BLINK_HALF + BLINK_CLOSED {
            Some(0)
        } else {
            Some(1)
        }
    }
}

pub fn prince_blinker() -> Blinker {
    Blinker::new(17, 1.3, 2.6, 3.4)
}

pub fn princess_blinker() -> Blinker {
    Blinker::new(91, 2.9, 3.1, 3.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_entries_in_order_only_single_player_enabled() {
        let labels: Vec<_> = MenuItem::ALL.iter().map(|i| i.label()).collect();
        assert_eq!(labels, ["Tek Oyunculu", "Çok Oyunculu", "Ayarlar"]);
        let enabled: Vec<_> = MenuItem::ALL.iter().map(|i| i.enabled()).collect();
        assert_eq!(enabled, [true, false, false]);
        for item in MenuItem::ALL {
            assert_eq!(item.enabled(), item.disabled_hint().is_none());
        }
    }

    #[test]
    fn navigation_wraps_and_starts_on_single_player() {
        let mut m = MenuModel::default();
        assert_eq!(m.selected_item(), MenuItem::SinglePlayer);
        m.navigate(1);
        assert_eq!(m.selected_item(), MenuItem::Multiplayer);
        m.navigate(1);
        assert_eq!(m.selected_item(), MenuItem::Settings);
        m.navigate(1);
        assert_eq!(m.selected_item(), MenuItem::SinglePlayer);
        m.navigate(-1);
        assert_eq!(m.selected_item(), MenuItem::Settings);
        m.select(7);
        assert_eq!(m.selected_item(), MenuItem::Settings);
    }

    #[test]
    fn disabled_entries_never_activate() {
        let mut m = MenuModel::default();
        assert_eq!(m.activate_selected(), Some(MenuAction::StartSinglePlayer));
        m.navigate(1);
        assert_eq!(m.activate_selected(), None);
        m.navigate(1);
        assert_eq!(m.activate_selected(), None);
        assert_eq!(m.activate(1), None);
        assert_eq!(m.activate(2), None);
        assert_eq!(m.activate(99), None);
    }

    #[test]
    fn loop_frame_depends_on_time_not_frame_rate() {
        // Same wall-clock instant → same frame, whatever the sampling rate.
        for fps_render in [30.0, 60.0, 144.0, 17.0] {
            let dt = 1.0 / fps_render;
            let steps = (2.5 / dt) as u32;
            let t = steps as f64 * dt;
            assert_eq!(loop_frame(t, 8.0, 40), loop_frame(t, 8.0, 40));
        }
        assert_eq!(loop_frame(0.0, 8.0, 40), 0);
        assert_eq!(loop_frame(0.124, 8.0, 40), 0);
        assert_eq!(loop_frame(0.126, 8.0, 40), 1);
        assert_eq!(loop_frame(5.0, 8.0, 40), 0); // 5 s loop
        assert_eq!(
            loop_frame(3600.0 + 0.2, 12.0, 16),
            loop_frame(0.2, 12.0, 16)
        );
    }

    #[test]
    fn blinks_are_irregular_and_not_in_lockstep() {
        let mut a = prince_blinker();
        let mut b = princess_blinker();
        let dt = 1.0 / 120.0;
        let (mut blinks_a, mut blinks_b, mut both_closed, mut closed_a) = (0, 0, 0, 0);
        let (mut prev_a, mut prev_b) = (None, None);
        let mut starts = Vec::new();
        let mut t = 0.0f32;
        while t < 300.0 {
            let fa = a.frame(t);
            let fb = b.frame(t);
            if fa.is_some() && prev_a.is_none() {
                blinks_a += 1;
                starts.push(t);
            }
            if fb.is_some() && prev_b.is_none() {
                blinks_b += 1;
            }
            if fa == Some(1) {
                closed_a += 1;
                if fb == Some(1) {
                    both_closed += 1;
                }
            }
            prev_a = fa;
            prev_b = fb;
            t += dt;
        }
        // Natural-ish rate: one blink every ~2.5–7 s.
        for n in [blinks_a, blinks_b] {
            assert!((40..=130).contains(&n), "blinks {n}");
        }
        // Rarely closed together.
        assert!(both_closed * 10 < closed_a, "{both_closed} / {closed_a}");
        // Gaps vary (not a metronome).
        let gaps: Vec<f32> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        let min = gaps.iter().cloned().fold(f32::MAX, f32::min);
        let max = gaps.iter().cloned().fold(0.0, f32::max);
        assert!(max - min > 2.0, "gaps {min}..{max}");
    }

    #[test]
    fn blink_sequence_is_half_closed_half() {
        let mut b = Blinker::new(1, 1.0, 3.0, 0.0);
        assert_eq!(b.frame(0.99), None);
        assert_eq!(b.frame(1.01), Some(0));
        assert_eq!(b.frame(1.08), Some(1));
        assert_eq!(b.frame(1.16), Some(0));
        assert_eq!(b.frame(1.3), None);
    }
}
