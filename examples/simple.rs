use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    input::mouse::MouseWheel, prelude::*,
};
use bevy_egui::*;
use solis_2d::prelude::*;

pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "radiance cascade".into(),
                    ..default()
                }),
                ..default()
            }),
            bevy_egui::EguiPlugin,
            SolisPlugin::default(),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, (setup, spawn_info_box))
        .add_systems(
            Update,
            (scroll, move_camera, move_light, spawn_light, clear, config),
        )
        .run();
}

#[derive(Component)]
struct FollowMouse;

#[derive(Component)]
struct Keep;

fn setup(mut cmd: Commands) {
    cmd.spawn((
        Camera2d,
        Camera {
            clear_color: Color::BLACK.into(),
            hdr: true,
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
                .looking_at(Vec3::default(), Vec3::Y),
        RadianceConfig::default(),
    ));

    cmd.spawn((
        Visibility::default(),
        Transform::default(),
        Emitter {
            intensity: 1.,
            color: Color::BLACK,
            shape: SdfShape::Rect(Vec2::new(50., 25.)),
        },
        Keep,
    ));
    cmd.spawn((
        Emitter {
            intensity: 1.,
            color: Color::WHITE,
            shape: SdfShape::Circle(200.),
        },
        FollowMouse,
        Visibility::default(),
        Transform::default(),
    ));
}

fn config(mut gi_config: Query<&mut RadianceConfig>, mut egui: EguiContexts) {
    let Ok(mut cfg) = gi_config.get_single_mut() else {
        return;
    };

    egui::Window::new("Gi Config")
        .anchor(egui::Align2::RIGHT_TOP, [0., 0.])
        .show(egui.ctx_mut(), |ui| {
            ui.label("probe stride");
            ui.add(egui::Slider::new(&mut cfg.probe_base, (1)..=16));
            ui.label("cascade count");
            ui.add(egui::Slider::new(&mut cfg.cascade_count, (2)..=8));
            ui.label("interval");
            ui.add(egui::Slider::new(&mut cfg.interval, (0.1)..=10.));
            ui.label("scale");
            ui.add(egui::Slider::new(&mut cfg.scale_factor, (0.25)..=10.));
            ui.label("edge highlight");
            ui.add(egui::Slider::new(&mut cfg.edge_hightlight, (0.0)..=100.));
            ui.label("light hight");
            ui.add(egui::Slider::new(&mut cfg.light_z, (-5.)..=50.));

            ui.label("modulate");
            let mut rgb = to_egui_color(cfg.modulate);
            egui::color_picker::color_edit_button_rgba(
                ui,
                &mut rgb,
                egui::color_picker::Alpha::BlendOrAdditive,
            );
            cfg.modulate = to_bevy_color(rgb);

            ui.label("absorb");
            rgb = to_egui_color(cfg.absorb);
            egui::color_picker::color_edit_button_rgba(
                ui,
                &mut rgb,
                egui::color_picker::Alpha::BlendOrAdditive,
            );
            cfg.absorb = to_bevy_color(rgb);

            flag_checkbox(GiFlags::DEBUG_SDF, ui, &mut cfg.flags, "SDF");
            flag_checkbox(GiFlags::DEBUG_VORONOI, ui, &mut cfg.flags, "VORONOI");
            flag_checkbox(
                GiFlags::OCCLUDER_LIGHT,
                ui,
                &mut cfg.flags,
                "OCCLUDER_NORMAL",
            );
        });
}


fn to_bevy_color(color: egui::Rgba) -> LinearRgba {
    LinearRgba::from_f32_array(color.to_array())
}
fn to_egui_color(color: LinearRgba) -> egui::Rgba {
    egui::Rgba::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
}

fn flag_checkbox(bit: GiFlags, ui: &mut egui::Ui, flags: &mut GiFlags, label: &str) {
    let mut state = (*flags & bit) != GiFlags::DEFAULT;
    ui.checkbox(&mut state, label);
    if state {
        *flags |= bit;
    } else {
        *flags &= !bit;
    }
}

fn spawn_light(
    mut cmd: Commands,
    mut light: Query<(&mut Emitter, &Transform), With<FollowMouse>>,
    inputs: Res<ButtonInput<MouseButton>>,
    mut egui: EguiContexts,
    server: Res<AssetServer>,
) {
    let Ok((mut emitter, transform)) = light.get_single_mut() else {
        return;
    };

    if egui.ctx_mut().is_pointer_over_area() {
        return;
    }

    if inputs.just_pressed(MouseButton::Left) {
        cmd.spawn((
            emitter.clone(),
            Sprite::from_image(server.load("lamp.png")),
            transform.clone(),
        ));

        let color = Color::srgb(
            0.8, 0.,
            0.,
            // rand::random::<f32>(),
            // rand::random::<f32>(),
            // rand::random::<f32>(),
        );

        emitter.color = color;
    }

    if inputs.just_pressed(MouseButton::Right) {
        cmd.spawn((
            Emitter {
                shape: emitter.shape.clone(),
                color: Color::BLACK,
                intensity: 1.,
            },
            Sprite::from_image(server.load("lamp.png")),
            transform.clone(),
        ));
    }
}

fn clear(
    mut cmd: Commands,
    emitters: Query<Entity, (Without<FollowMouse>, With<Emitter>, Without<Keep>)>,
    inputs: Res<ButtonInput<KeyCode>>,
) {
    if inputs.just_pressed(KeyCode::KeyR) {
        emitters
            .iter()
            .for_each(|entity| cmd.entity(entity).despawn_recursive());
    }
}

fn scroll(
    mut emitters: Query<&mut Emitter, With<FollowMouse>>,
    mut events: EventReader<MouseWheel>,
    inputs: Res<ButtonInput<KeyCode>>,
    mut multi: Local<f32>,
) {
    let Some(event) = events.read().next() else {
        return;
    };

    let Ok(mut emitter) = emitters.get_single_mut() else {
        return;
    };

    let dir = event.y.signum();
    match inputs.pressed(KeyCode::ShiftLeft) {
        true => {
            emitter.intensity = (emitter.intensity + dir * 0.1).max(0.);
        }
        false => {
            *multi = (*multi + dir).max(0.);
            emitter.shape = SdfShape::Circle(*multi);
        }
    }
}

fn move_camera(mut camera: Query<&mut Transform, With<Camera>>, inputs: Res<ButtonInput<KeyCode>>) {
    let Ok(mut transform) = camera.get_single_mut() else {
        return;
    };

    transform.translation += Vec3::new(
        (inputs.pressed(KeyCode::ArrowRight) as i32 - inputs.pressed(KeyCode::ArrowLeft) as i32)
            as f32,
        (inputs.pressed(KeyCode::ArrowUp) as i32 - inputs.pressed(KeyCode::ArrowDown) as i32)
            as f32,
        0.,
    ) * 2.;

    if inputs.just_pressed(KeyCode::KeyO) {
        transform.scale += Vec3::splat(0.05);
    }
    if inputs.just_pressed(KeyCode::KeyI) {
        transform.scale -= Vec3::splat(0.05);
    }
}

fn move_light(
    window: Query<&Window>,
    camera: Query<Entity, With<Camera2d>>,
    light: Query<Entity, With<FollowMouse>>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position().map(|pos| {
        let offset = transforms
            .get(camera.single())
            .unwrap()
            .translation
            .truncate();
        (pos - Vec2::new(window.width() / 2., window.height() / 2.)) * Vec2::new(1., -1.) + offset
    }) else {
        return;
    };

    let scale = transforms.get(camera.single()).unwrap().scale.x;
    let Some(mut light_transform) = light
        .get_single()
        .ok()
        .map(|entity| transforms.get_mut(entity).ok())
        .flatten()
    else {
        return;
    };

    light_transform.translation.x = cursor_pos.x / scale;
    light_transform.translation.y = cursor_pos.y / scale;
}

fn spawn_info_box(mut cmd: Commands) {

    let mut node = Node::DEFAULT;
    node.width = Val::Percent(100.);
    node.height = Val::Percent(100.);
    node.align_items = AlignItems::End;
    node.justify_content = JustifyContent::Start;

    cmd.spawn(node).with_children(|cmd| {

        let mut node = Node::DEFAULT;

        node.border = UiRect::all(Val::Px(4.));
        node.padding = UiRect::all(Val::Px(10.));

        cmd.spawn(node)
        .insert(BackgroundColor(Color::BLACK))
        .insert(BorderRadius::all(Val::Px(15.)))
        .with_children(|cmd| {

            cmd.spawn((
                Text::new(
                    "[Arrowkeys]:Move [I]:Zoom-in [O]:Zoom-out [Wheel]:Inc-size [Wheel+shift]:Inc-intensity [R]: clear screen"
                ),
                TextFont{
                    font_size: 20.,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}
