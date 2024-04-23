use bevy::app::AppExit;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::input::common_conditions::*;
use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::time::common_conditions::on_timer;
use rand::Rng;
use std::time::Duration;

/**
 * Default values.
 */
const CONNECT_FORCE: f32 = 300.;
const SPEED: f32 = 1.;
const DOT_SIZE: f32 = 6.;
const MIN_VEL: f32 = -600.;
const MAX_VEL: f32 = 600.;

const INFO_TEXT_PADDING: Val = Val::Px(6.0);
const INFO_TEXT_SIZE: f32 = 16.;
const INFO_TEXT_COLOR: Color = Color::ANTIQUE_WHITE;

const DRAG_SPAWN_INTERVAL: u64 = 70; // In ms

// Used to identify the Dots
#[derive(Component)]
struct Dot;

// The info text
#[derive(Component)]
struct InfoText;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

// Associated to the gizmos line for the line connecting the dots
#[derive(Default, Reflect, GizmoConfigGroup)]
struct Lines {}

// Variables of the simulation
#[derive(Resource)]
struct SimuConf {
    dot_size: f32,
    speed: f32,
    connect_force: f32,
    min_vel: f32,
    max_vel: f32,
    freeze_dots: bool,
    number_of_dots: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .init_gizmo_group::<Lines>()
        .insert_resource(SimuConf {
            dot_size: DOT_SIZE,
            speed: SPEED,
            min_vel: MIN_VEL,
            max_vel: MAX_VEL,
            connect_force: CONNECT_FORCE,
            freeze_dots: false,
            number_of_dots: 0,
        })
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            spawn_dots_on_cursor
                .run_if(on_timer(Duration::from_millis(DRAG_SPAWN_INTERVAL)))
                .run_if(input_pressed(MouseButton::Left)),
        )
        .add_systems(
            Update,
            clear_dots.run_if(input_just_pressed(KeyCode::Space)),
        )
        .add_systems(
            Update,
            (
                handle_keyboard_input,
                update_info_text,
                (apply_dot_velocity, apply_dot_collision, connect_dot).chain(),
            ),
        )
        .run();
}

fn distance_between_points(p1: Vec2, p2: Vec2) -> f32 {
    ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt()
}

fn map(value: f32, from_low: f32, from_high: f32, to_low: f32, to_high: f32) -> f32 {
    return to_low + (to_high - to_low) * ((value - from_low) / (from_high - from_low));
}

fn connect_dot(
    mut gizmos: Gizmos<Lines>,
    query: Query<&Transform, With<Dot>>,
    simu_conf: Res<SimuConf>,
) {
    for [dot, dot2] in query.iter_combinations() {
        let d1 = Vec2::new(dot.translation.x, dot.translation.y);
        let d2 = Vec2::new(dot2.translation.x, dot2.translation.y);
        let dist = distance_between_points(d1, d2);
        if dist < simu_conf.connect_force {
            let alpha = map(dist, 0., simu_conf.connect_force, 1., 0.);
            let color = Color::rgba(0.93, 0.51, 0.93, alpha);
            gizmos.line_2d(d1, d2, color);
        }
    }
}

fn clear_dots(
    mut query: Query<Entity, With<Dot>>,
    mut commands: Commands,
    mut simu_conf: ResMut<SimuConf>,
) {
    for dot in &mut query {
        commands.entity(dot).despawn();
    }
    simu_conf.number_of_dots = 0;
}

fn spawn_dots_on_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut simu_conf: ResMut<SimuConf>,
) {
    let (camera, camera_transform) = camera_query.single();

    let Some(cursor_position) = windows.single().cursor_position() else {
        return;
    };

    // Calculate a world position based on the cursor's position.
    let Some(cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    let mut rng = rand::thread_rng();
    let r_x = rng.gen_range(simu_conf.min_vel..simu_conf.max_vel) as f32;
    let r_y = rng.gen_range(simu_conf.min_vel..simu_conf.max_vel) as f32;

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes
                .add(Circle {
                    radius: simu_conf.dot_size,
                })
                .into(),
            // transform: Transform::default().with_scale(Vec3::splat(10.)).,
            transform: Transform::from_xyz(cursor_pos.x, cursor_pos.y, 1.),
            material: materials.add(Color::VIOLET),
            ..default()
        },
        Dot,
        Velocity(Vec2::new(r_x, r_y)),
    ));

    simu_conf.number_of_dots += 1;
}

fn apply_dot_velocity(
    mut query: Query<(&mut Transform, &Velocity)>,
    time: Res<Time>,
    simu_conf: ResMut<SimuConf>,
) {
    if simu_conf.freeze_dots {
        return;
    }

    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * simu_conf.speed * time.delta_seconds();
        transform.translation.y += velocity.y * simu_conf.speed * time.delta_seconds();
    }
}

fn apply_dot_collision(
    mut query: Query<(&mut Transform, &mut Velocity), With<Dot>>,
    window: Query<&Window>,
) {
    let window = window.single();
    let width = window.resolution.width();
    let height = window.resolution.height();

    let ratio = 2.;

    for (mut transform, mut velocity) in &mut query {
        if transform.translation.x >= width / ratio {
            velocity.x = -velocity.x;
            transform.translation.x = width / ratio;
        } else if transform.translation.x <= -width / ratio {
            velocity.x = -velocity.x;
            transform.translation.x = -width / ratio;
        }

        if transform.translation.y >= height / ratio {
            velocity.y = -velocity.y;
            transform.translation.y = height / ratio;
        } else if transform.translation.y <= -height / ratio {
            velocity.y = -velocity.y;
            transform.translation.y = -height / ratio;
        }
    }
}

fn handle_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut simu_conf: ResMut<SimuConf>,
    mut writer: EventWriter<AppExit>,
) {
    if keyboard_input.pressed(KeyCode::KeyI) {
        simu_conf.connect_force += 2.;
    }

    if keyboard_input.pressed(KeyCode::KeyK) {
        simu_conf.connect_force -= 2.;
    }

    if keyboard_input.pressed(KeyCode::KeyU) {
        simu_conf.speed += 0.04;
    }

    if keyboard_input.pressed(KeyCode::KeyJ) {
        simu_conf.speed -= 0.04;
    }

    if keyboard_input.just_pressed(KeyCode::KeyR) {
        simu_conf.speed *= -1.;
    }

    if keyboard_input.just_pressed(KeyCode::KeyP) {
        simu_conf.freeze_dots = !simu_conf.freeze_dots;
    }

    if keyboard_input.pressed(KeyCode::Escape) {
        writer.send(AppExit);
    }
}

fn update_info_text(simu_conf: Res<SimuConf>, mut query: Query<&mut Text, With<InfoText>>) {
    let mut text = query.single_mut();
    let info_text = format!(
        "Dot (Click/Space): {} | Connect Force (I/K) : {} | Speed (U/J): {}",
        simu_conf.number_of_dots, simu_conf.connect_force, simu_conf.speed
    );
    text.sections[0].value = info_text;
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        InfoText,
        TextBundle::from(TextSection::new(
            "info text",
            TextStyle {
                font_size: INFO_TEXT_SIZE,
                color: INFO_TEXT_COLOR,
                ..default()
            },
        ))
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: INFO_TEXT_PADDING,
            left: INFO_TEXT_PADDING,
            ..default()
        }),
    ));
}
