//! Minimal HUD: status line, controls hint, toasts, death flash, victory and
//! pause panels. Plain UI text and colored boxes only.

use bevy::prelude::*;
use game_core as gc;

use crate::GameSim;
use crate::effects::{BestTime, FLASH_DURATION, ScreenFlash, Toast};

const TEXT: Color = Color::srgb(0.92, 0.92, 0.95);
const DIM: Color = Color::srgba(0.85, 0.87, 0.92, 0.75);
const ACCENT: Color = Color::srgb(1.0, 0.85, 0.35);
const GOOD: Color = Color::srgb(0.45, 0.95, 0.55);
const PANEL: Color = Color::srgba(0.04, 0.05, 0.08, 0.78);
const STRIP: Color = Color::srgba(0.04, 0.05, 0.08, 0.55);

#[derive(Component)]
pub struct StatusText;

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

fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
    )
}

/// Full-screen container that places its panel in the upper part of the
/// screen, leaving the hero (below the center) visible.
fn centered_panel() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::FlexStart,
        padding: UiRect::top(Val::Percent(16.0)),
        ..default()
    }
}

fn panel_box() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Px(28.0), Val::Px(18.0)),
        row_gap: Val::Px(8.0),
        ..default()
    }
}

pub fn spawn_hud(mut commands: Commands) {
    commands.insert_resource(Toast::default());
    commands.insert_resource(ScreenFlash::default());
    commands.insert_resource(BestTime::default());

    // Death flash (full screen, behind the text).
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

    // Status line (top-left).
    commands.spawn((
        StatusText,
        label("", 16.0, TEXT),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Px(6.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(STRIP),
    ));

    // Controls hint (bottom-left).
    commands.spawn((
        label(
            "Move A/D or Left/Right   Jump Space (hold = higher)   \
             Wall jump: jump while touching a wall   R restart   Esc pause",
            13.0,
            DIM,
        ),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            bottom: Val::Px(6.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(STRIP),
    ));

    // Toast (upper middle).
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Px(56.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((ToastText, label("", 20.0, GOOD)));
        });

    // Victory panel.
    commands
        .spawn((VictoryPanel, centered_panel(), Visibility::Hidden))
        .with_children(|p| {
            p.spawn((panel_box(), BackgroundColor(PANEL)))
                .with_children(|b| {
                    b.spawn(label("Course complete!", 28.0, ACCENT));
                    b.spawn((VictoryText, label("", 18.0, TEXT)));
                    b.spawn(label("Press R to play again", 15.0, DIM));
                });
        });

    // Pause panel.
    commands
        .spawn((PausePanel, centered_panel(), Visibility::Hidden))
        .with_children(|p| {
            p.spawn((panel_box(), BackgroundColor(PANEL)))
                .with_children(|b| {
                    b.spawn(label("Paused", 28.0, TEXT));
                    b.spawn(label("Esc resume   R restart level", 15.0, DIM));
                });
        });
}

pub fn format_time(seconds: f32) -> String {
    let total_cs = (seconds.max(0.0) * 100.0).round() as u32;
    let m = total_cs / 6000;
    let s = (total_cs / 100) % 60;
    let cs = total_cs % 100;
    format!("{m}:{s:02}.{cs:02}")
}

type StatusQ<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<StatusText>, Without<ToastText>, Without<VictoryText>)>;
type ToastQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<ToastText>, Without<StatusText>, Without<VictoryText>),
>;
type VictoryTextQ<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<VictoryText>, Without<StatusText>, Without<ToastText>)>;
type PanelsQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Visibility, Has<VictoryPanel>, Has<PausePanel>),
    Or<(With<VictoryPanel>, With<PausePanel>)>,
>;

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    sim: Res<GameSim>,
    toast: Res<Toast>,
    flash: Res<ScreenFlash>,
    best: Res<BestTime>,
    mut status: StatusQ,
    mut toast_text: ToastQ,
    mut victory_text: VictoryTextQ,
    mut panels: PanelsQ,
    mut overlay: Query<&mut BackgroundColor, With<FlashOverlay>>,
) {
    let s = &sim.0;
    let cps = s.level.checkpoints.len();
    let cp = s.active_checkpoint.map_or(0, |i| i + 1);

    if let Ok(mut text) = status.single_mut() {
        let line = format!(
            "Time {}    Deaths {}    Checkpoint {}/{}",
            format_time(s.elapsed),
            s.deaths,
            cp,
            cps
        );
        if text.0 != line {
            text.0 = line;
        }
    }

    if let Ok((mut text, mut color)) = toast_text.single_mut() {
        let want = if toast.remaining > 0.0 {
            toast.text.as_str()
        } else {
            ""
        };
        if text.0 != want {
            text.0 = want.to_string();
        }
        // Fade out over the last half second.
        color.0.set_alpha((toast.remaining / 0.5).clamp(0.0, 1.0));
    }

    if let Ok(mut text) = victory_text.single_mut() {
        let best = best.0.map_or_else(|| "-".to_string(), format_time);
        let line = format!(
            "Time {}    Best {}    Deaths {}",
            format_time(s.elapsed),
            best,
            s.deaths
        );
        if text.0 != line {
            text.0 = line;
        }
    }

    for (mut vis, is_victory, is_pause) in &mut panels {
        let show = (is_victory && s.phase == gc::Phase::Completed) || (is_pause && s.is_paused());
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }

    if let Ok(mut bg) = overlay.single_mut() {
        let a = 0.45 * (flash.remaining / FLASH_DURATION).clamp(0.0, 1.0);
        bg.0 = Color::srgba(0.85, 0.10, 0.12, a);
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
    }
}
