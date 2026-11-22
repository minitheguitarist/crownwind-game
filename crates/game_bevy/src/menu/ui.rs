//! Menu UI: logo, three translucent buttons, hint lines, fade overlay.
//!
//! Laid out in the menu camera's 640×360 viewport with one UI unit = one
//! render pixel (see `fit_menu_view`), so the bitmap labels and the logo
//! land on the same pixel grid as the scene. Buttons are real UI nodes:
//! hover selects, click / Enter activates — disabled entries never do.

use bevy::asset::RenderAssetUsages;
use bevy::picking::events::{PointerClick, PointerOver};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::camp_layout::LOGO_SIZE;
use super::font_data::{SANS_SMALL, SERIF};
use super::model::{MenuAction, MenuItem, MenuModel};
use super::pixel_font::{Bitmap, PixelFont};
use super::{AppState, FadeOverlay, MenuArt, MenuCamera, MenuTransition, request_start};

const BUTTON_W: f32 = 188.0;
const BUTTON_H: f32 = 26.0;

// Night-blue glass with antique-gold trim.
const GLASS: Color = Color::srgba(0.06, 0.09, 0.18, 0.45);
const GLASS_ACTIVE: Color = Color::srgba(0.10, 0.14, 0.27, 0.72);
const GLASS_OFF: Color = Color::srgba(0.05, 0.07, 0.12, 0.30);
const GLASS_OFF_ACTIVE: Color = Color::srgba(0.07, 0.09, 0.16, 0.48);
const TRIM: Color = Color::srgba(0.85, 0.68, 0.40, 0.45);
const TRIM_ACTIVE: Color = Color::srgba(0.97, 0.81, 0.48, 1.0);
const TRIM_OFF: Color = Color::srgba(0.55, 0.58, 0.66, 0.25);
const TRIM_OFF_ACTIVE: Color = Color::srgba(0.64, 0.68, 0.78, 0.6);
const INK: Color = Color::srgb(0.95, 0.91, 0.80);
const INK_ACTIVE: Color = Color::srgb(1.0, 0.87, 0.55);
const INK_OFF: Color = Color::srgba(0.56, 0.59, 0.67, 0.85);
const INK_OFF_ACTIVE: Color = Color::srgb(0.68, 0.72, 0.80);
const INK_HINT: Color = Color::srgba(0.74, 0.78, 0.86, 0.9);
const GOLD: [u8; 4] = [246, 206, 122, 255];

#[derive(Component)]
pub struct MenuButton(pub usize);
#[derive(Component)]
pub struct ButtonLabel(pub usize);
#[derive(Component)]
pub struct ButtonMarker(pub usize);
#[derive(Component)]
pub struct DisabledHint;

/// Pre-rendered "why disabled" captions, one per menu entry.
#[derive(Resource)]
pub struct HintImages(pub Vec<Option<(Handle<Image>, Vec2)>>);

pub fn bitmap_image(b: &Bitmap) -> Image {
    Image::new(
        Extent3d {
            width: b.width,
            height: b.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        b.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn text_node(
    images: &mut Assets<Image>,
    font: &PixelFont,
    text: &str,
    tint: Color,
) -> (ImageNode, Node) {
    let bmp = font.render(text);
    let size = Vec2::new(bmp.width as f32, bmp.height as f32);
    let handle = images.add(bitmap_image(&bmp));
    (
        ImageNode {
            color: tint,
            ..ImageNode::new(handle)
        },
        Node {
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            ..default()
        },
    )
}

/// 5×5 gold diamond marking the highlighted entry.
fn diamond() -> Bitmap {
    let mut rgba = vec![0u8; 5 * 5 * 4];
    for y in 0..5i32 {
        for x in 0..5i32 {
            if (x - 2).abs() + (y - 2).abs() <= 2 {
                let i = ((y * 5 + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&GOLD);
            }
        }
    }
    Bitmap {
        width: 5,
        height: 5,
        rgba,
    }
}

pub fn spawn_menu_ui(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut art: ResMut<MenuArt>,
    camera: Query<Entity, With<MenuCamera>>,
) {
    let Ok(cam) = camera.single() else {
        error!("menu camera missing");
        return;
    };
    commands.insert_resource(MenuModel::default());

    let logo: Handle<Image> = assets.load("menu/logo.png");
    art.0.push(logo.clone().untyped());
    let marker = images.add(bitmap_image(&diamond()));

    let hints = MenuItem::ALL
        .iter()
        .map(|item| {
            item.disabled_hint().map(|text| {
                let bmp = SANS_SMALL.render(text);
                let size = Vec2::new(bmp.width as f32, bmp.height as f32);
                (images.add(bitmap_image(&bmp)), size)
            })
        })
        .collect();
    commands.insert_resource(HintImages(hints));

    let root = commands
        .spawn((
            DespawnOnExit(AppState::MainMenu),
            UiTargetCamera(cam),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // Left column: logo, buttons, disabled hint.
    let column = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(62.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        ImageNode::new(logo),
        Node {
            width: Val::Px(LOGO_SIZE.0 as f32),
            height: Val::Px(LOGO_SIZE.1 as f32),
            margin: UiRect::bottom(Val::Px(26.0)),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(column),
    ));

    let buttons = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                margin: UiRect::left(Val::Px(14.0)),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(column),
        ))
        .id();

    for (index, item) in MenuItem::ALL.into_iter().enumerate() {
        let button = commands
            .spawn((
                MenuButton(index),
                Node {
                    width: Val::Px(BUTTON_W),
                    height: Val::Px(BUTTON_H),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(GLASS),
                BorderColor::all(TRIM),
                ChildOf(buttons),
            ))
            .observe(
                move |_over: On<PointerOver>, mut model: ResMut<MenuModel>| {
                    if model.selected != index {
                        model.select(index);
                    }
                },
            )
            .observe(
                move |click: On<PointerClick>,
                      model: Res<MenuModel>,
                      mut transition: ResMut<MenuTransition>| {
                    if click.button != PointerButton::Primary {
                        return;
                    }
                    // Disabled entries return `None`: the click does nothing.
                    if let Some(MenuAction::StartSinglePlayer) = model.activate(index) {
                        request_start(&mut transition);
                    }
                },
            )
            .id();

        // Glass highlight along the top edge.
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(1.0),
                right: Val::Px(1.0),
                top: Val::Px(0.0),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)),
            Pickable::IGNORE,
            ChildOf(button),
        ));
        // Selection diamond, just left of the button.
        commands.spawn((
            ButtonMarker(index),
            ImageNode::new(marker.clone()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-11.0),
                top: Val::Px(((BUTTON_H - 2.0 - 5.0) / 2.0).floor()),
                width: Val::Px(5.0),
                height: Val::Px(5.0),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
            ChildOf(button),
        ));
        let (img, node) = text_node(&mut images, &SERIF, item.label(), INK);
        commands.spawn((
            ButtonLabel(index),
            img,
            node,
            Pickable::IGNORE,
            ChildOf(button),
        ));
        if !item.enabled() {
            let tag = commands
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(3.0),
                            Val::Px(2.0),
                            Val::Px(2.0),
                            Val::Px(1.0),
                        ),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(TRIM_OFF_ACTIVE),
                    BackgroundColor(Color::srgba(0.04, 0.05, 0.09, 0.35)),
                    Pickable::IGNORE,
                    ChildOf(button),
                ))
                .id();
            let (img, node) = text_node(&mut images, &SANS_SMALL, "YAKINDA", INK_OFF_ACTIVE);
            commands.spawn((img, node, Pickable::IGNORE, ChildOf(tag)));
        }
    }

    // Caption explaining a highlighted disabled entry.
    commands.spawn((
        DisabledHint,
        ImageNode::default(),
        Node {
            margin: UiRect::new(Val::Px(16.0), Val::Px(0.0), Val::Px(9.0), Val::Px(0.0)),
            width: Val::Px(1.0),
            height: Val::Px(1.0),
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
        ChildOf(column),
    ));

    // Controls hint, bottom-left.
    let (img, node) = text_node(
        &mut images,
        &SANS_SMALL,
        "↑↓  Seç      Enter  Onayla",
        INK_HINT,
    );
    commands.spawn((
        img,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(42.0),
            bottom: Val::Px(18.0),
            ..node
        },
        Pickable::IGNORE,
        ChildOf(root),
    ));

    // Fade overlay: black at first, cleared by the intro fade, black again
    // when "Tek Oyunculu" is activated.
    commands.spawn((
        FadeOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        GlobalZIndex(50),
        Pickable::IGNORE,
        ChildOf(root),
    ));
}

/// Arrow keys / W-S move the highlight; Enter or Space runs it if enabled.
pub fn menu_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut model: ResMut<MenuModel>,
    mut transition: ResMut<MenuTransition>,
) {
    if transition.started {
        return;
    }
    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        model.navigate(-1);
    }
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        model.navigate(1);
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space])
        && let Some(MenuAction::StartSinglePlayer) = model.activate_selected()
    {
        request_start(&mut transition);
    }
}

type ButtonQ<'w, 's> = Query<
    'w,
    's,
    (
        &'static MenuButton,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
>;

pub fn refresh_buttons(
    model: Res<MenuModel>,
    hints: Option<Res<HintImages>>,
    mut buttons: ButtonQ,
    mut labels: Query<(&ButtonLabel, &mut ImageNode), Without<DisabledHint>>,
    mut markers: Query<(&ButtonMarker, &mut Visibility), Without<DisabledHint>>,
    mut hint: Query<(&mut ImageNode, &mut Node, &mut Visibility), With<DisabledHint>>,
) {
    if !model.is_changed() {
        return;
    }
    for (b, mut bg, mut border) in &mut buttons {
        let item = MenuItem::ALL[b.0];
        let sel = model.selected == b.0;
        let (g, t) = match (item.enabled(), sel) {
            (true, false) => (GLASS, TRIM),
            (true, true) => (GLASS_ACTIVE, TRIM_ACTIVE),
            (false, false) => (GLASS_OFF, TRIM_OFF),
            (false, true) => (GLASS_OFF_ACTIVE, TRIM_OFF_ACTIVE),
        };
        bg.0 = g;
        *border = BorderColor::all(t);
    }
    for (l, mut img) in &mut labels {
        let item = MenuItem::ALL[l.0];
        img.color = match (item.enabled(), model.selected == l.0) {
            (true, false) => INK,
            (true, true) => INK_ACTIVE,
            (false, false) => INK_OFF,
            (false, true) => INK_OFF_ACTIVE,
        };
    }
    for (m, mut vis) in &mut markers {
        *vis = if model.selected == m.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let (Some(hints), Ok((mut img, mut node, mut vis))) = (hints, hint.single_mut()) {
        match &hints.0[model.selected] {
            Some((handle, size)) => {
                img.image = handle.clone();
                img.color = INK_HINT;
                node.width = Val::Px(size.x);
                node.height = Val::Px(size.y);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn menu_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<MenuModel>()
            .init_resource::<MenuTransition>()
            .add_systems(Update, menu_keyboard);
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
        keys.press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    #[test]
    fn keyboard_cannot_start_disabled_entries() {
        let mut app = menu_app();
        press(&mut app, KeyCode::ArrowDown); // Çok Oyunculu
        assert_eq!(app.world().resource::<MenuModel>().selected, 1);
        press(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<MenuTransition>().started);
        press(&mut app, KeyCode::ArrowDown); // Ayarlar
        assert_eq!(app.world().resource::<MenuModel>().selected, 2);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Space);
        assert!(!app.world().resource::<MenuTransition>().started);
    }

    #[test]
    fn keyboard_starts_single_player() {
        let mut app = menu_app();
        press(&mut app, KeyCode::ArrowUp); // wraps to Ayarlar
        assert_eq!(app.world().resource::<MenuModel>().selected, 2);
        press(&mut app, KeyCode::ArrowDown); // wraps to Tek Oyunculu
        assert_eq!(app.world().resource::<MenuModel>().selected, 0);
        press(&mut app, KeyCode::Enter);
        assert!(app.world().resource::<MenuTransition>().started);
    }

    #[test]
    fn diamond_is_symmetric() {
        let d = diamond();
        for y in 0..5 {
            for x in 0..5 {
                assert_eq!(d.pixel(x, y), d.pixel(4 - x, y));
            }
        }
    }
}
