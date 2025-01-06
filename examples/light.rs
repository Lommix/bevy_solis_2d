use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::MouseWheel,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    window::WindowResized,
    image::ImageSampler,
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
            (
                update,
                scroll,
                move_camera,
                move_light,
                spawn_light,
                clear,
                config,
                diagnostics,
                monitor,
                sync_size,
            ),
        )
        .run();
}

#[derive(Component)]
struct Spin(f32);

#[derive(Component)]
struct FollowMouse;

#[derive(Component)]
struct MainCamera;

fn monitor(diagnostics: Res<DiagnosticsStore>) {
    let Some(_fps) = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .map(|fps| fps.value())
        .flatten()
    else {
        return;
    };
}

fn create_image(size: Vec2) -> Image {
    let size = Extent3d {
        width: size.x as u32,
        height: size.y as u32,
        depth_or_array_layers: 1,
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        sampler: ImageSampler::nearest(),
        ..default()
    };
    image.resize(size);
    image
}

// since the normals are rendered to a texture with
// another camera, we have to sync the target size with
// the current window size by hand
fn sync_size(
    mut events: EventReader<WindowResized>,
    normals: Query<&NormalTarget>,
    window: Query<&Window>,
    mut images: ResMut<Assets<Image>>,
) {
    if events.read().count() == 0 {
        return;
    }

    let Ok(win_size) = window.get_single().map(|win| win.size()) else {
        return;
    };

    normals.iter().for_each(|n| {
        images.insert(&n.0, create_image(win_size));
    });
}

fn setup(mut cmd: Commands, server: Res<AssetServer>, mut images: ResMut<Assets<Image>>) {
    let image_handle = images.add(create_image(Vec2::new(1024., 1024.)));
    cmd.spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
                .looking_at(Vec3::default(), Vec3::Y),
        Camera {
            clear_color: Color::BLACK.into(),
            hdr: true,
            ..default()
        },
        Tonemapping::AcesFitted,
        RadianceConfig::default(),
        MainCamera,
        NormalTarget(image_handle.clone()),
    ));
    cmd.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Image(image_handle),
            ..default()
        },
        RenderLayers::layer(3),
    ));

    for x in -4..=8 {
        for y in -4..=8 {
            let rand = rand::random::<f32>();
            cmd.spawn((
                Sprite::from_image(server.load("box.png")),
                Transform::from_xyz((x as f32) * 400., (y as f32) * 400., 1.),
                Emitter {
                    intensity: 0.0,
                    color: Color::BLACK,
                    shape: SdfShape::Rect(Vec2::new(50., 25.)),
                },
                Spin(rand),
            ));

            cmd.spawn((
                Sprite::from_image(server.load("boxn.png")),
                Transform::from_xyz((x as f32) * 400., (y as f32) * 400., 1.),
                RenderLayers::layer(3),
                Spin(rand),
            ));
        }
    }

    let map_size = 8;
    for x in -map_size..map_size {
        for y in -map_size..map_size {
            let ox = x as f32 * 2048.;
            let oy = y as f32 * 2048.;

            cmd.spawn((
                Sprite::from_image(server.load("brick.png")),
                Transform::from_translation(Vec3::new(ox, oy, 0.)),
            ));
            cmd.spawn((
                Sprite::from_image(server.load("brickn.png")),
                Transform::from_translation(Vec3::new(ox, oy, 0.)),
                RenderLayers::layer(3),
            ));
        }
    }

    cmd.spawn((
        Emitter {
            intensity: 1.,
            color: Color::WHITE,
            shape: SdfShape::Circle(200.),
        },
        FollowMouse,
        Sprite::from_image(server.load("lamp.png")),
        Transform::from_translation(Vec3::new(0., 0., 0.)),
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

#[derive(Default)]
struct Timings {
    count: u32,
    acc_fps: f64,
    acc_frame: f64,
    avg_fps: f64,
    avg_frame: f64,
}

fn diagnostics(
    mut egui: EguiContexts,
    diagnostics: Res<DiagnosticsStore>,
    mut timings: Local<Timings>,
) {
    let Some(fps_time) = diagnostics.get_measurement(&FrameTimeDiagnosticsPlugin::FPS) else {
        return;
    };

    let Some(frame_time) = diagnostics.get_measurement(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
    else {
        return;
    };

    timings.acc_fps += fps_time.value;
    timings.acc_frame += frame_time.value;
    timings.count += 1;

    if timings.count == 100 {
        timings.avg_fps = timings.acc_fps * 0.01;
        timings.avg_frame = timings.acc_frame * 0.01;
        timings.count = 0;
        timings.acc_fps = 0.;
        timings.acc_frame = 0.;
    }

    egui::Window::new("Diagnostics")
        .anchor(egui::Align2::RIGHT_BOTTOM, [0., 0.])
        .show(egui.ctx_mut(), |ui| {
            ui.label(format!("FPS: {:.3}", timings.avg_fps));
            ui.label(format!("FRAME: {:.2}ms", timings.avg_frame));
        });
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

fn update(mut query: Query<(&mut Transform, &Spin)>, time: Res<Time>) {
    query.iter_mut().for_each(|(mut transform, spin)| {
        transform.rotation = Quat::from_rotation_z(time.delta_secs() * spin.0);
    });
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
            rand::random::<f32>(),
            rand::random::<f32>(),
            rand::random::<f32>(),
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
    emitters: Query<Entity, (Without<FollowMouse>, Without<Spin>, With<Emitter>)>,
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
            emitter.intensity = emitter.intensity + dir.signum() * 0.2;
        }
        false => {
            *multi = (*multi + dir).max(0.);
            emitter.shape = SdfShape::Circle(*multi);
        }
    }
}

fn move_camera(mut camera: Query<&mut Transform, With<Camera>>, inputs: Res<ButtonInput<KeyCode>>) {
    camera.iter_mut().for_each(|mut transform| {
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
    });
}

fn move_light(
    window: Query<&Window>,
    camera: Query<Entity, (With<Camera2d>, With<MainCamera>)>,
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
