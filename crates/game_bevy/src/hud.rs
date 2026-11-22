//! Minimal HUD: status bar, loading notice, a start-of-play controls guide
//! that fades out once the player moves, toasts, death flash, and the
//! victory / pause panels. Plain UI text and colored boxes only.

use bevy::prelude::*;
use game_core as gc;

use crate::GameSim;
use crate::effects::{BestTime, FLASH_DURATION, ScreenFlash, Toast};
use crate::readiness::SceneReady;

const TEXT: Color = Color::srgb(0.95, 0.93, 0.88);
const LABEL: Color = Color::srgb(0.66, 0.74, 0.80);
const DIM: Color = Color::srgb(0.80, 0.82, 0.86);
const ACCENT: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
const GOOD: Color = Color::srgb_u8(0xF2, 0xC8, 0xA0);
const PANEL: Color = Color::srgba(0.09, 0.07, 0.12, 0.82);
const BAR: Color = Color::srgba(0.09, 0.07, 0.12, 0.72);

/// Seconds the start guide takes to fade out once dismissed.
pub const GUIDE_FADE: f32 = 0.6;
/// Distance (px) the prince must move from the spawn to dismiss the guide.
pub const GUIDE_DISMISS_DISTANCE: f32 = 24.0;

#[derive(Component)]
pub struct TimeValue;
#[derive(Component)]
pub struct DeathsValue;
#[derive(Component)]
pub struct CheckpointValue;
#[derive(Component)]
pub struct ToastText;
#[derive(Component)]
pub struct FlashOverlay;
#[derive(Component)]
pub struct VictoryPanel;
#[derive(Component)]
pub struct VictoryText;
#[derive(Component)]
pub struct PausePanel;
#[derive(Component)]
pub struct LoadingPanel;
#[derive(Component)]
pub struct StatusBar;

/// The start-of-play controls guide (background box).
#[derive(Component)]
pub struct GuidePanel;
/// Text inside the guide; keeps its full-opacity color for fading.
#[derive(Component)]
pub struct GuideText(Color);

/// Visibility state of the start guide.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub dismissed: bool,
    pub alpha: f32,
}

impl Default for Guide {
    fn default() -> Self {
        Self {
            dismissed: false,
            alpha: 1.0,
        }
    }
}

impl Guide {
    /// Pure update: dismiss once the player has clearly started playing,
    /// then fade out over [`GUIDE_FADE`] seconds.
    pub fn step(&mut self, moved_from_spawn: f32, dt: f32) {
        if !self.dismissed && moved_from_spawn > GUIDE_DISMISS_DISTANCE {
            self.dismissed = true;
        }
        if self.dismissed {
            self.alpha = (self.alpha - dt.max(0.0) / GUIDE_FADE).max(0.0);
        }
    }
}

fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
    )
}

/// Full-screen layout container. Note: percentage padding resolves against
/// the parent's *width* (CSS rules), so offsets here are in pixels.
fn full_screen(align: AlignItems, padding: UiRect) -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: align,
        padding,
        ..default()
    }
}

fn panel_box() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Px(28.0), Val::Px(16.0)),
        row_gap: Val::Px(8.0),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }
}

/// "Label value" pair in the status bar.
fn stat(parent: &mut ChildSpawnerCommands, name: &str, marker: impl Component) {
    parent
        .spawn(Node {
            column_gap: Val::Px(6.0),
            align_items: AlignItems::Baseline,
            ..default()
        })
        .with_children(|p| {
            p.spawn(label(name, 13.0, LABEL));
            p.spawn((marker, label("", 18.0, TEXT)));
        });
}

const CONTROLS: [(&str, &str); 4] = [
    ("Move", "A / D  or  Left / Right"),
    ("Jump", "Space  (hold to jump higher)"),
    ("Wall jump", "jump while sliding on a wall"),
    ("Restart / Pause", "R  /  Esc"),
];

fn controls_table(parent: &mut ChildSpawnerCommands, guide: bool) {
    for (what, keys) in CONTROLS {
        parent
            .spawn(Node {
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|row| {
                if guide {
                    row.spawn((label(what, 16.0, ACCENT), GuideText(ACCENT)));
                    row.spawn((label(keys, 16.0, TEXT), GuideText(TEXT)));
                } else {
                    row.spawn(label(what, 16.0, ACCENT));
                    row.spawn(label(keys, 16.0, TEXT));
                }
            });
    }
}

pub fn spawn_hud(mut commands: Commands) {
    commands.insert_resource(Toast::default());
    commands.insert_resource(ScreenFlash::default());
    commands.insert_resource(BestTime::default());
    commands.insert_resource(Guide::default());

    // Death flash (full screen, behind everything else).
    commands.spawn((
        FlashOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));

    // Status bar (top-left): TIME · DEATHS · CHECKPOINT.
    commands
        .spawn((
            StatusBar,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                column_gap: Val::Px(20.0),
                align_items: AlignItems::Baseline,
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(BAR),
            Visibility::Hidden,
        ))
        .with_children(|bar| {
            stat(bar, "TIME", TimeValue);
            stat(bar, "DEATHS", DeathsValue);
            stat(bar, "CHECKPOINT", CheckpointValue);
        });

    // Start guide (lower middle), fades once the player starts moving.
    commands
        .spawn(full_screen(
            AlignItems::FlexEnd,
            UiRect::bottom(Val::Px(36.0)),
        ))
        .with_children(|p| {
            p.spawn((
                GuidePanel,
                panel_box(),
                BackgroundColor(PANEL),
                Visibility::Hidden,
            ))
            .with_children(|b| controls_table(b, true));
        });

    // Toast (upper middle).
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Px(60.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((ToastText, label("", 20.0, GOOD)));
        });

    // Victory panel.
    commands
        .spawn((
            VictoryPanel,
            full_screen(AlignItems::FlexStart, UiRect::top(Val::Px(110.0))),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((panel_box(), BackgroundColor(PANEL)))
                .with_children(|b| {
                    b.spawn(label("Course complete!", 28.0, ACCENT));
                    b.spawn((VictoryText, label("", 18.0, TEXT)));
                    b.spawn(label("Press R to play again", 15.0, DIM));
                });
        });

    // Pause panel (also the place to re-read the controls).
    commands
        .spawn((
            PausePanel,
            full_screen(AlignItems::FlexStart, UiRect::top(Val::Px(70.0))),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((panel_box(), BackgroundColor(PANEL)))
                .with_children(|b| {
                    b.spawn(label("Paused", 28.0, TEXT));
                    controls_table(b, false);
                    b.spawn(label("Esc to resume", 15.0, DIM));
                });
        });

    // Loading notice, until the scene can actually be drawn.
    commands
        .spawn((LoadingPanel, full_screen(AlignItems::Center, UiRect::ZERO)))
        .with_children(|p| {
            p.spawn(label("Loading Windwood...", 18.0, DIM));
        });
}

pub fn format_time(seconds: f32) -> String {
    let total_cs = (seconds.max(0.0) * 100.0).round() as u32;
    let m = total_cs / 6000;
    let s = (total_cs / 100) % 60;
    let cs = total_cs % 100;
    format!("{m}:{s:02}.{cs:02}")
}

fn set_text(text: &mut Text, value: String) {
    if text.0 != value {
        text.0 = value;
    }
}

fn set_vis(vis: &mut Visibility, show: bool) {
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
}

/// Start guide: dismiss on movement, fade, and apply alpha.
pub fn update_guide(
    time: Res<Time>,
    sim: Res<GameSim>,
    scene: Res<SceneReady>,
    mut guide: ResMut<Guide>,
    mut panel: Query<(&mut Visibility, &mut BackgroundColor), With<GuidePanel>>,
    mut texts: Query<(&GuideText, &mut TextColor)>,
) {
    if !scene.is_ready() {
        return;
    }
    let p = sim.0.player.position;
    let s = sim.0.level.spawn;
    let moved = ((p.x - s.x).powi(2) + (p.y - s.y).powi(2)).sqrt();
    let dt = if sim.0.is_paused() {
        0.0
    } else {
        time.delta_secs()
    };
    guide.step(moved, dt);
    let a = guide.alpha;
    // Hidden under the pause panel so the two never overlap.
    let show = a > 0.0 && !sim.0.is_paused();
    if let Ok((mut vis, mut bg)) = panel.single_mut() {
        set_vis(&mut vis, show);
        bg.0 = PANEL.with_alpha(PANEL.alpha() * a);
    }
    for (base, mut color) in &mut texts {
        color.0 = base.0.with_alpha(base.0.alpha() * a);
    }
}

type Values<'w, 's> = (
    Query<
        'w,
        's,
        &'static mut Text,
        (
            With<TimeValue>,
            Without<DeathsValue>,
            Without<CheckpointValue>,
        ),
    >,
    Query<
        'w,
        's,
        &'static mut Text,
        (
            With<DeathsValue>,
            Without<TimeValue>,
            Without<CheckpointValue>,
        ),
    >,
    Query<
        'w,
        's,
        &'static mut Text,
        (
            With<CheckpointValue>,
            Without<TimeValue>,
            Without<DeathsValue>,
        ),
    >,
);

type Others<'w, 's> = (
    Query<
        'w,
        's,
        (&'static mut Text, &'static mut TextColor),
        (
            With<ToastText>,
            Without<TimeValue>,
            Without<DeathsValue>,
            Without<CheckpointValue>,
            Without<VictoryText>,
        ),
    >,
    Query<
        'w,
        's,
        &'static mut Text,
        (
            With<VictoryText>,
            Without<TimeValue>,
            Without<DeathsValue>,
            Without<CheckpointValue>,
            Without<ToastText>,
        ),
    >,
);

type Panels<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Visibility,
        Has<VictoryPanel>,
        Has<PausePanel>,
        Has<LoadingPanel>,
        Has<StatusBar>,
    ),
    Or<(
        With<VictoryPanel>,
        With<PausePanel>,
        With<LoadingPanel>,
        With<StatusBar>,
    )>,
>;

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    sim: Res<GameSim>,
    scene: Res<SceneReady>,
    toast: Res<Toast>,
    flash: Res<ScreenFlash>,
    best: Res<BestTime>,
    mut values: ParamSet<Values>,
    mut others: ParamSet<Others>,
    mut panels: Panels,
    mut overlay: Query<&mut BackgroundColor, With<FlashOverlay>>,
) {
    let s = &sim.0;
    let cps = s.level.checkpoints.len();
    let cp = s.active_checkpoint.map_or(0, |i| i + 1);

    if let Ok(mut t) = values.p0().single_mut() {
        set_text(&mut t, format_time(s.elapsed));
    }
    if let Ok(mut t) = values.p1().single_mut() {
        set_text(&mut t, s.deaths.to_string());
    }
    if let Ok(mut t) = values.p2().single_mut() {
        set_text(&mut t, format!("{cp} / {cps}"));
    }

    if let Ok((mut text, mut color)) = others.p0().single_mut() {
        let want = if toast.remaining > 0.0 {
            toast.text.clone()
        } else {
            String::new()
        };
        set_text(&mut text, want);
        color.0.set_alpha((toast.remaining / 0.5).clamp(0.0, 1.0));
    }
    if let Ok(mut text) = others.p1().single_mut() {
        let best = best.0.map_or_else(|| "-".to_string(), format_time);
        set_text(
            &mut text,
            format!(
                "Time {}    Best {}    Deaths {}",
                format_time(s.elapsed),
                best,
                s.deaths
            ),
        );
    }

    let ready = scene.is_ready();
    for (mut vis, victory, pause, loading, bar) in &mut panels {
        let show = (victory && s.phase == gc::Phase::Completed)
            || (pause && s.is_paused() && ready)
            || (loading && !ready)
            || (bar && ready);
        set_vis(&mut vis, show);
    }

    if let Ok(mut bg) = overlay.single_mut() {
        let a = 0.45 * (flash.remaining / FLASH_DURATION).clamp(0.0, 1.0);
        bg.0 = Color::srgba(0.60, 0.16, 0.24, a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_format() {
        assert_eq!(format_time(0.0), "0:00.00");
        assert_eq!(format_time(61.234), "1:01.23");
        assert_eq!(format_time(9.999), "0:10.00");
        assert_eq!(format_time(76.55), "1:16.55");
    }

    #[test]
    fn guide_stays_until_player_moves_then_fades() {
        let mut g = Guide::default();
        for _ in 0..600 {
            g.step(3.0, 1.0 / 60.0); // idle / tiny shuffles
        }
        assert_eq!(g.alpha, 1.0);
        assert!(!g.dismissed);
        g.step(GUIDE_DISMISS_DISTANCE + 1.0, 0.0);
        assert!(g.dismissed);
        g.step(0.0, GUIDE_FADE * 0.5);
        assert!((g.alpha - 0.5).abs() < 1e-5);
        // Walking back to the spawn does not bring it back.
        g.step(0.0, GUIDE_FADE);
        assert_eq!(g.alpha, 0.0);
        g.step(0.0, 1.0);
        assert_eq!(g.alpha, 0.0);
    }
}
