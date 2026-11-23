use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use rand::prelude::*;
use std::time::Duration;

const SNAKE_HEAD_COLOR: Color = Color::linear_rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::linear_rgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::linear_rgb(1.0, 0.0, 1.0);
const ARENA_WIDTH: u32 = 50;
const ARENA_HEIGHT: u32 = 50;

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}
#[derive(Component)]
struct SnakeSegment;
#[derive(Default, Resource, Deref, DerefMut)]
struct SnakeSegments(Vec<Entity>);
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}
#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);
#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}
#[derive(Component)]
struct Food;
#[derive(Message)]
struct GrowthEvent;
#[derive(Message)]
struct GameOverEvent;
#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

// ============== main ==============

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.0)))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_message::<GrowthEvent>()
        .add_message::<GameOverEvent>()
        .add_systems(Startup, (setup_camera, spawn_snake))
        .add_systems(
            Update,
            (
                snake_movement_input.before(snake_movement),
                snake_movement.run_if(on_timer(Duration::from_millis(150))),
                game_over.after(snake_movement),
                snake_eating.after(snake_movement),
                snake_growth.after(snake_eating),
                food_spawner.run_if(on_timer(Duration::from_secs(1))),
            ),
        )
        .add_systems(PostUpdate, (position_translation, size_scaling))
        .add_plugins(DefaultPlugins)
        .run();
}

// ======== Startup systems =========

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_snake(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut segments: ResMut<SnakeSegments>,
) {
    *segments = SnakeSegments(vec![
        commands
            .spawn((
                Mesh2d(meshes.add(Rectangle::default())),
                MeshMaterial2d(materials.add(SNAKE_HEAD_COLOR)),
                Transform::default().with_scale(Vec3::splat(10.)),
            ))
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(Size::square(0.8))
            .id(),
        spawn_segment(commands, meshes, materials, Position { x: 3, y: 2 }),
    ]);
}

// ======== PostUpdate systems =========

fn size_scaling(windows: Query<&Window>, mut query: Query<(&Size, &mut Transform)>) {
    if let Ok(window) = windows.single() {
        let window_width = window.resolution.width();
        let window_height = window.resolution.height();

        for (mesh_size, transform) in query.iter_mut() {
            let _ = transform.with_scale(Vec3::new(
                mesh_size.width / ARENA_WIDTH as f32 * window_width,
                mesh_size.height / ARENA_HEIGHT as f32 * window_height,
                1.0,
            ));
        }
    }
}

fn position_translation(windows: Query<&Window>, mut query: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }

    if let Ok(window) = windows.single() {
        for (pos, mut transform) in query.iter_mut() {
            transform.translation = Vec3::new(
                convert(pos.x as f32, window.width(), ARENA_WIDTH as f32),
                convert(pos.y as f32, window.height(), ARENA_HEIGHT as f32),
                0.0,
            )
        }
    }
}

// ======== Update systems =========

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::KeyH)
            || keyboard_input.pressed(KeyCode::ArrowLeft)
        {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::KeyL)
            || keyboard_input.pressed(KeyCode::ArrowRight)
        {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::KeyJ)
            || keyboard_input.pressed(KeyCode::ArrowDown)
        {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::KeyK) || keyboard_input.pressed(KeyCode::ArrowUp)
        {
            Direction::Up
        } else {
            head.direction
        };

        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_writer: MessageWriter<GameOverEvent>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut positions: Query<&mut Position>,
) {
    if let Some((head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();

        match &head.direction {
            Direction::Left => {
                head_pos.x -= 1;
            }
            Direction::Right => {
                head_pos.x += 1;
            }
            Direction::Up => {
                head_pos.y += 1;
            }
            Direction::Down => {
                head_pos.y -= 1;
            }
        };

        if head_pos.x < 0
            || head_pos.y < 0
            || head_pos.x as u32 >= ARENA_WIDTH
            || head_pos.y as u32 >= ARENA_HEIGHT
        {
            game_over_writer.write(GameOverEvent);
        }

        if segment_positions.contains(&head_pos) {
            game_over_writer.write(GameOverEvent);
        }

        segment_positions
            .iter()
            .zip(segments.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });

        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
    }
}

fn food_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::rng();

    commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(FOOD_COLOR)),
            Transform::default().with_scale(Vec3::splat(10.)),
        ))
        .insert(Food)
        .insert(Position {
            x: (rng.random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (rng.random::<f32>() * ARENA_HEIGHT as f32) as i32,
        })
        .insert(Size::square(0.8));
}

fn spawn_segment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    position: Position,
) -> Entity {
    commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(SNAKE_SEGMENT_COLOR)),
            Transform::default().with_scale(Vec3::splat(10.)),
        ))
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(0.65))
        .id()
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: MessageWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_postions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_postions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.write(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: MessageReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        segments.push(spawn_segment(
            commands,
            meshes,
            materials,
            last_tail_position.0.unwrap(),
        ));
    }
}

fn game_over(
    mut commands: Commands,
    mut reader: MessageReader<GameOverEvent>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
) {
    if reader.read().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, meshes, materials, segments_res);
    }
}
