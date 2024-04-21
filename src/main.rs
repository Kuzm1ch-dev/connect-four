use bevy::{input::common_conditions::input_just_pressed, prelude::*, window::PrimaryWindow};

use std::collections::{hash_map, HashMap, HashSet};

#[derive(Copy, Clone)]
enum ElementType {
    Red = 0,
    Blue = 1,
}

#[derive(Component, Debug)]
struct Grid {
    width: u32,
    height: u32,
    elements: HashMap<UVec2, u32>,
}

enum MatchDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub enum Match {
    /// A straight match of 3 or more gems
    Straight(HashSet<UVec2>),
}

#[derive(Default, Clone)]
pub struct Matches {
    matches: Vec<Match>,
}

enum ElemError {
    NoElem,
}

impl Matches {
    fn add(&mut self, mat: Match) {
        self.matches.push(mat)
    }

    fn append(&mut self, other: &mut Matches) {
        self.matches.append(&mut other.matches);
    }

    /// Returns the coordinates of all matches in this collection without any repeated values
    fn without_duplicates(&self) -> HashSet<UVec2> {
        self.matches
            .iter()
            .flat_map(|mat| match mat {
                Match::Straight(mat) => mat,
            })
            .cloned()
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

#[derive(Component)]
struct Element;

impl Grid {
    fn get(&self, pos: &UVec2) -> Result<&u32, ElemError> {
        let elem = self.elements.get(pos);
        if elem.is_none() {
            return Err((ElemError::NoElem));
        }
        return Ok((elem.unwrap()));
    }

    fn insert(&mut self, pos: UVec2, typ: u32) {
        self.elements.insert(pos, typ);
    }

    fn add_at_column(&mut self, column: u32, element_type: u32) {
        for y in 0..self.height {
            let pos = [column, y];
            if self.get(&pos.into()).is_err() {
                self.insert(pos.into(), element_type);
                return;
            }
        }
    }

    fn get_matches(&self) -> Matches {
        let mut matches = self.straight_matches(MatchDirection::Horizontal);
        matches.append(&mut self.straight_matches(MatchDirection::Vertical));
        matches
    }

    fn straight_matches(&self, direction: MatchDirection) -> Matches {
        let mut matches = Matches::default();
        let mut current_match = vec![];
        let mut previous_type = None;
        for one in match direction {
            MatchDirection::Horizontal => 0..self.width,
            MatchDirection::Vertical => 0..self.height,
        } {
            for two in match direction {
                MatchDirection::Horizontal => 0..self.height,
                MatchDirection::Vertical => 0..self.width,
            } {
                let pos = [
                    match direction {
                        MatchDirection::Horizontal => one,
                        MatchDirection::Vertical => two,
                    },
                    match direction {
                        MatchDirection::Horizontal => two,
                        MatchDirection::Vertical => one,
                    },
                ]
                .into();

                if let Ok(current_type) = self.get(&pos) {
                    if current_match.is_empty() || previous_type.unwrap() == current_type {
                        previous_type = Some(current_type);
                        current_match.push(pos);
                    } else if previous_type.unwrap() != current_type {
                        match current_match.len() {
                            0..=3 => {}
                            _ => matches
                                .add(Match::Straight(current_match.iter().cloned().collect())),
                        }
                        current_match = vec![pos];
                        previous_type = Some(current_type);
                    }
                }
            }
            match current_match.len() {
                0..=3 => {}
                _ => matches.add(Match::Straight(current_match.iter().cloned().collect())),
            }
            current_match = vec![];
            previous_type = None;
        }
        matches
    }
}

#[derive(Resource)]
struct CursorWorldPos(Option<Vec2>);

#[derive(Resource)]
struct Column(Option<u32>);

#[derive(Resource)]
struct Player(Option<u32>);

pub const YELLOW: Color = Color::rgb(1.0, 1.0, 0.0);

const ELEMENT_SIZE: f32 = 80.;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(CursorWorldPos(None))
        .insert_resource(Column(None))
        .insert_resource(Player(Some(0)))
        .add_systems(Startup, (setup).chain())
        .add_systems(
            Update,
            (
                get_cursor_world_pos,
                (
                    check_mouse_pos,
                    spawn_element
                        .run_if(input_just_pressed(MouseButton::Left))
                        .run_if(resource_exists::<Column>),
                    draw,
                )
                    .chain(),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    q_window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    let window = q_window.single();
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(window.width() / 2., window.height() / 2., 0.),
        ..default()
    });
    let grid = Grid {
        width: 7,
        height: 6,
        elements: HashMap::new(),
    };
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2 {
                    x: ELEMENT_SIZE * grid.width as f32,
                    y: ELEMENT_SIZE * grid.height as f32,
                }),
                ..default()
            },
            texture: asset_server.load("sprites/grid.png"),
            transform: Transform::from_xyz(window.width() / 2., window.height() / 2., 0.),
            ..default()
        },
        ImageScaleMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 1., // The image will tile every 128px
        },
        grid,
    ));
}

// Получаем координаты курсора и сохраняем как ресурс
fn get_cursor_world_pos(
    mut cursor_world_pos: ResMut<CursorWorldPos>,
    q_primary_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    let primary_window = q_primary_window.single();
    let (main_camera, main_camera_transform) = q_camera.single();
    cursor_world_pos.0 = primary_window
        .cursor_position()
        .and_then(|cursor_pos| main_camera.viewport_to_world_2d(main_camera_transform, cursor_pos));
}

fn check_mouse_pos(
    mut commands: Commands,
    cursor_world_pos: Res<CursorWorldPos>,
    mut column: ResMut<Column>,
    mut q_grid: Query<&mut Grid>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    let window = q_window.single();
    if let Ok(mut grid) = q_grid.get_single_mut() {
        let left_down_corner = Vec2 {
            x: window.width() / 2. - (grid.width as f32 / 2. * ELEMENT_SIZE),
            y: window.height() / 2. - (grid.height as f32 / 2. * ELEMENT_SIZE),
        };

        let right_up_corner = Vec2 {
            x: window.width() / 2. + (grid.width as f32 / 2. * ELEMENT_SIZE),
            y: window.height() / 2. + (grid.height as f32 / 2. * ELEMENT_SIZE),
        };
        if (!cursor_world_pos.0.is_none()) {
            let mouse_pos = cursor_world_pos.0.unwrap();
            if ((mouse_pos.x > left_down_corner.x && mouse_pos.x < right_up_corner.x)
                && (mouse_pos.y > left_down_corner.y && mouse_pos.y < right_up_corner.y))
            {
                let x_pos = (mouse_pos - left_down_corner).x;
                let selected_column = (x_pos as f32 / ELEMENT_SIZE) as i32;
                column.0 = Some(selected_column as u32);
            }
        }
    }
}

fn spawn_element(column: Res<Column>, mut player: ResMut<Player>, mut q_grid: Query<&mut Grid>) {
    if let Ok(mut grid) = q_grid.get_single_mut() {
        grid.add_at_column(column.0.unwrap(), player.0.unwrap());
        let matches = grid.get_matches();
        if !matches.matches.is_empty() && matches.matches.len() > 0 {
            println!("Победил {} игрок!!!", match player.0.unwrap() {
                0 => "красный",
                _ => "синий"
            });
        }
        player.0 = match player.0 {
            Some(0) => Some(1),
            _ => Some(0),
        };
        println!("Ход игрока {}", player.0.unwrap());
    }
}

fn draw(
    mut commands: Commands,
    mut q_grid: Query<&mut Grid>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_elements: Query<(&Element, Entity)>,
    asset_server: Res<AssetServer>,
) {
    let window = q_window.single();
    if let Ok(mut grid) = q_grid.get_single_mut() {
        let left_up_corner = Vec2 {
            x: window.width() / 2. - (grid.width as f32 / 2. * ELEMENT_SIZE),
            y: window.height() / 2. - (grid.height as f32 / 2. * ELEMENT_SIZE),
        };

        for (_, entity) in q_elements.iter() {
            commands.entity(entity).despawn();
        }
        for _y in 0..grid.height {
            for _x in 0..grid.width {
                if !grid.get(&[_x, _y].into()).is_err() {
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2 {
                                    x: ELEMENT_SIZE,
                                    y: ELEMENT_SIZE,
                                }),
                                ..default()
                            },
                            texture: asset_server.load(format!(
                                "sprites/{}.png",
                                grid.elements[&UVec2 { x: _x, y: _y }]
                            )),
                            transform: Transform::from_xyz(
                                (left_up_corner.x + ELEMENT_SIZE / 2.) + _x as f32 * ELEMENT_SIZE,
                                (left_up_corner.y + ELEMENT_SIZE / 2.) + _y as f32 * ELEMENT_SIZE,
                                1.,
                            ),
                            ..default()
                        },
                        Element,
                    ));
                }
            }
        }
    }
}
