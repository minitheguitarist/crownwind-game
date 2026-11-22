//! Windwood backdrop plates (sky, distant forest).
//!
//! Both plates are exactly one 480×270 view and are **not** horizontally
//! seamless, so they are never repeated naïvely:
//! - **Sky** is locked to the view (no parallax). It is the most distant
//!   layer, so a fixed sky reads as "infinitely far away".
//! - **Forest** scrolls horizontally at [`FOREST_PARALLAX`] using copies
//!   that alternate between normal and mirrored. A mirrored neighbour shares
//!   the exact same edge column, so there is no hard seam or gap. Vertically
//!   it stays locked to the view bottom (no edge can be exposed).
//!
//! Set `FOREST_PARALLAX` to 0.0 to lock the forest to the view as well.

use bevy::prelude::*;

use crate::VIRTUAL_WIDTH;
use crate::view::MainCamera;

pub const SKY_PATH: &str = "environment/windwood/sky.png";
pub const FOREST_PATH: &str = "environment/windwood/forest.png";

/// Fraction of camera motion the forest follows *relative to the view*
/// (0 = locked to the screen, 1 = fixed in the world).
pub const FOREST_PARALLAX: f32 = 0.25;

const Z_SKY: f32 = -100.0;
const Z_FOREST: f32 = -90.0;

#[derive(Component)]
pub struct SkyPlate;

/// Forest copy `k` (0..=2) of the mirrored strip.
#[derive(Component)]
pub struct ForestPlate(pub i32);

/// Horizontal placement of forest copy `k` for a camera at `camera_x`.
///
/// Returns (left edge of the copy relative to the view's left edge, whether
/// the copy is mirrored). The pattern repeats every two plates
/// (normal + mirrored), so copies 0..=2 always cover the 480 px view.
pub fn forest_copy(camera_x: f32, k: i32) -> (f32, bool) {
    let w = VIRTUAL_WIDTH;
    let scroll = (camera_x * FOREST_PARALLAX).round();
    let local = scroll.rem_euclid(2.0 * w);
    (k as f32 * w - local, k.rem_euclid(2) == 1)
}

pub fn spawn_backdrop(mut commands: Commands, assets: Res<AssetServer>) {
    let sky: Handle<Image> = assets.load(SKY_PATH);
    let forest: Handle<Image> = assets.load(FOREST_PATH);
    commands.spawn((
        SkyPlate,
        Sprite::from_image(sky),
        Transform::from_xyz(0.0, 0.0, Z_SKY),
    ));
    for k in 0..=2 {
        commands.spawn((
            ForestPlate(k),
            Sprite::from_image(forest.clone()),
            Transform::from_xyz(0.0, 0.0, Z_FOREST),
        ));
    }
}

type CameraQ<'w, 's> =
    Query<'w, 's, &'static Transform, (With<MainCamera>, Without<SkyPlate>, Without<ForestPlate>)>;

/// Runs after the camera moved: keep plates aligned with the (snapped) view.
pub fn follow_backdrop(
    camera: CameraQ,
    mut sky: Query<&mut Transform, (With<SkyPlate>, Without<ForestPlate>)>,
    mut forest: Query<(&ForestPlate, &mut Transform, &mut Sprite), Without<SkyPlate>>,
) {
    let Ok(cam) = camera.single() else {
        return;
    };
    let c = cam.translation;
    if let Ok(mut t) = sky.single_mut() {
        t.translation.x = c.x;
        t.translation.y = c.y;
    }
    let view_left = c.x - VIRTUAL_WIDTH * 0.5;
    for (plate, mut t, mut sprite) in &mut forest {
        let (left, mirrored) = forest_copy(c.x, plate.0);
        t.translation.x = view_left + left + VIRTUAL_WIDTH * 0.5;
        // Same size as the view: centered on it = bottom edges coincide.
        t.translation.y = c.y;
        if sprite.flip_x != mirrored {
            sprite.flip_x = mirrored;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three copies must tile [0, 480) of the view with no gap/overlap.
    fn covered(camera_x: f32) -> bool {
        let mut spans: Vec<(f32, f32)> = (0..=2)
            .map(|k| {
                let (l, _) = forest_copy(camera_x, k);
                (l, l + VIRTUAL_WIDTH)
            })
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let contiguous = spans.windows(2).all(|w| w[0].1 == w[1].0);
        let first = spans[0].0;
        let last = spans[2].1;
        contiguous && first <= 0.0 && last >= VIRTUAL_WIDTH
    }

    #[test]
    fn forest_copies_always_cover_the_view() {
        for i in -400..4000 {
            let x = i as f32 * 1.37;
            assert!(covered(x), "gap at camera x {x}");
        }
    }

    #[test]
    fn neighbouring_copies_alternate_mirroring() {
        for x in [0.0, 123.0, 2000.0, -50.0] {
            let m: Vec<bool> = (0..=2).map(|k| forest_copy(x, k).1).collect();
            assert_eq!(m, vec![false, true, false]);
        }
    }

    #[test]
    fn copies_land_on_whole_pixels() {
        for i in 0..500 {
            let (l, _) = forest_copy(i as f32 * 0.731, 1);
            assert_eq!(l.fract(), 0.0);
        }
    }

    #[test]
    fn mirrored_seam_continuity() {
        // A plate and its mirror image meet at a shared edge column: the
        // last column of the normal plate equals the first column of the
        // mirrored one (column 479 on both sides), so no hard seam exists.
        let (left0, m0) = forest_copy(0.0, 0);
        let (left1, m1) = forest_copy(0.0, 1);
        assert!(!m0 && m1);
        assert_eq!(left1 - left0, VIRTUAL_WIDTH);
    }
}
