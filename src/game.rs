use crate::{
    big_text::{blue_scored, blue_won, dots, draw, red_scored, red_won, BigNumberFont},
    types::*,
    utils::*,
};
use crossterm::event::KeyCode;
use image::{Rgba, RgbaImage};
use once_cell::sync::Lazy;
use rand::Rng;
use ratatui::{
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::Color,
    text::Line,
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

const MINIMUM_DELTATIME_MILLISECONDS: f32 = 18.0;
const GAME_DURATION_MILLISECONDS: u128 = 90 * 1000;
const STARTING_DELAY_MILLISECONDS: u128 = 3000;
const AFTER_GOAL_DELAY_MILLISECONDS: u128 = 2000;
const ENDING_DELAY_MILLISECONDS: u128 = 1000;

const MIN_X: f32 = 3.0;
const MAX_X: f32 = 157.0;
const MIN_Y: f32 = 3.0;
const MAX_Y: f32 = 83.0;

const GOALIE_AREA_WIDTH: f32 = 8.0;
const GOALIE_AREA_HEIGHT: f32 = 26.0;
const GOALIE_AREA_MIN_Y: f32 = 31.0;
const GOALIE_AREA_MAX_Y: f32 = 55.0;

const RED_INITIAL_POSITION: (f32, f32) = (20.0, 40.0);
const BLUE_INITIAL_POSITION: (f32, f32) = (132.0, 40.0);

const PUCK_WIDTH: f32 = 2.0;
const PUCK_HEIGHT: f32 = 2.0;
const GOALIE_WIDTH: f32 = 6.0;
const GOALIE_HEIGHT: f32 = 7.0;

const GOALIE_MIN_Y: f32 = 31.0;
const GOALIE_MAX_Y: f32 = 48.0;

const ACCELERATION: f32 = 0.2;
const DECELERATION: f32 = 0.4;
const MAX_PLAYER_VELOCITY: f32 = 1.3;
const MAX_PUCK_VELOCITY: f32 = 2.2;

const GOALIE_MASS: f32 = 1000.0;
const PLAYER_MASS: f32 = 20.0;
const PUCK_MASS: f32 = 1.0;

const PLAYER_FRICTION_VELOCITY_LOSS: f32 = 0.975;
const PUCK_FRICTION_VELOCITY_LOSS: f32 = 0.99;
const COEFFICIENT_OF_RESTITUTION: f32 = 0.7;
const COFFICIENT_OF_WALL_BOUNCING: f32 = 0.25;

const SKATE_TRACE_LENGTH: usize = 512;

const SHOOTING_COUNTER_MILLISECONDS: f32 = 350.0;
const AFTER_SHOOTING_COUNTER_MILLISECONDS: f32 = 50.0;
const AFTER_GOT_STOLEN_COUNTER_MILLISECONDS: f32 = 50.0;
const SHOOTING_DIRECTION_MODIFIER: f32 = 0.35;
const SHOOTING_POWER: f32 = 3.0;

static PITCH_EMPTY: Lazy<RgbaImage> =
    Lazy::new(|| read_image("pitch_empty.png").expect("Could not read pitch_empty.png."));

static PITCH_CLASSIC: Lazy<RgbaImage> =
    Lazy::new(|| read_image("pitch_classic.png").expect("Could not read pitch_classic.png."));

static PITCH_BASKET: Lazy<RgbaImage> =
    Lazy::new(|| read_image("pitch_basket.png").expect("Could not read pitch_basket.png."));

static PITCH_ALT: Lazy<RgbaImage> =
    Lazy::new(|| read_image("pitch_alt.png").expect("Could not read pitch_alt.png."));

static PUCK_DARK: Lazy<RgbaImage> =
    Lazy::new(|| read_image("puck_white.png").expect("Could not read puck.png."));

static PUCK_LIGHT: Lazy<RgbaImage> =
    Lazy::new(|| read_image("puck_black.png").expect("Could not read puck.png."));

static PUCK_GOLD: Lazy<RgbaImage> =
    Lazy::new(|| read_image("puck_gold.png").expect("Could not read puck.png."));

static RED_PLAYER: Lazy<Vec<RgbaImage>> = Lazy::new(|| {
    let mut images = vec![];
    for i in 1..=8 {
        images.push(
            read_image(format!("red{i}.png").as_str())
                .expect(format!("Could not read red{i}.png.").as_str()),
        );
    }
    images
});

static RED_GOALIE: Lazy<RgbaImage> =
    Lazy::new(|| read_image("red_goalie.png").expect("Could not read red_goalie.png."));

static BLUE_PLAYER: Lazy<Vec<RgbaImage>> = Lazy::new(|| {
    let mut images = vec![];
    for i in 1..=8 {
        images.push(
            read_image(format!("blue{i}.png").as_str())
                .expect(format!("Could not read blue{i}.png.").as_str()),
        );
    }
    images
});

static BLUE_GOALIE: Lazy<RgbaImage> =
    Lazy::new(|| read_image("blue_goalie.png").expect("Could not read blue_goalie.png."));

fn base_image(palette: Palette) -> RgbaImage {
    match palette {
        Palette::Dark => PITCH_EMPTY.clone(),
        Palette::Light => PITCH_CLASSIC.clone(),
        Palette::Basket => PITCH_BASKET.clone(),
        Palette::Alt => PITCH_ALT.clone(),
    }
}

fn skate_trace_color(palette: Palette) -> Rgba<u8> {
    match palette {
        Palette::Dark => Rgba([55, 55, 85, 255]),
        Palette::Light => Rgba([195, 195, 255, 255]),
        Palette::Basket => Rgba([55, 55, 85, 255]),
        Palette::Alt => Rgba([105, 55, 55, 255]),
    }
}

fn puck_catcher_offset(orientation: Orientation) -> (f32, f32) {
    match orientation {
        Orientation::Up => (18.0, 0.0),
        Orientation::UpLeft => (12.0, -2.0),
        Orientation::Left => (0.0, 0.0),
        Orientation::DownLeft => (-2.0, 1.0),
        Orientation::Down => (0.0, 6.0),
        Orientation::DownRight => (1.0, 15.0),
        Orientation::Right => (6.0, 18.0),
        Orientation::UpRight => (15.0, 12.0),
    }
}

/// Simple collision detection using rectangles.
/// Returns a boolean indicating if the two sprites are colliding.
fn are_sprites_colliding(rect1: Rect, rect2: Rect) -> bool {
    rect1.x < rect2.x + rect2.width
        && rect1.x + rect1.width > rect2.x
        && rect1.y < rect2.y + rect2.height
        && rect1.y + rect1.height > rect2.y
}

#[derive(Clone, Copy, PartialEq)]
enum Orientation {
    Up,
    UpLeft,
    Left,
    DownLeft,
    Down,
    DownRight,
    Right,
    UpRight,
}

impl Orientation {
    pub fn next(self) -> Self {
        ((self as u8 + 1) % 8).into()
    }

    pub fn previous(self) -> Self {
        ((self as u8 + 7) % 8).into()
    }
}

impl From<u8> for Orientation {
    fn from(value: u8) -> Self {
        match value {
            0 => Orientation::Up,
            1 => Orientation::UpLeft,
            2 => Orientation::Left,
            3 => Orientation::DownLeft,
            4 => Orientation::Down,
            5 => Orientation::DownRight,
            6 => Orientation::Right,
            7 => Orientation::UpRight,
            _ => panic!("Invalid orientation"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    // TODO: add character selection with different stats
    Starting { time: Instant },
    Running,
    AfterGoal { time: Instant, scored: GameSide },
    Ending { time: Instant },
}

enum CollisionType {
    Minimal,
    Full,
}
#[derive(Clone, Copy, PartialEq)]
enum Palette {
    Dark,
    Light,
    Basket,
    Alt,
}

impl Palette {
    pub fn next(&self) -> Self {
        match self {
            Palette::Dark => Palette::Light,
            Palette::Light => Palette::Basket,
            Palette::Basket => Palette::Alt,
            Palette::Alt => Palette::Dark,
        }
    }
}

fn resolve_collision(
    sprite1: &mut impl Body,
    sprite2: &mut impl Body,
    collision_type1: CollisionType,
    collision_type2: CollisionType,
) -> bool {
    let rect1 = match collision_type1 {
        CollisionType::Minimal => sprite1.minimal_collision_rect(),
        CollisionType::Full => sprite1.full_collision_rect(),
    };

    let rect2 = match collision_type2 {
        CollisionType::Minimal => sprite2.minimal_collision_rect(),
        CollisionType::Full => sprite2.full_collision_rect(),
    };

    // Check collisions between players
    if are_sprites_colliding(rect1, rect2) {
        // Calculate new velocities by conservation of momentum
        // Energy is dissipated in the collision by a factor ENERGY_LOSS
        let velocity_com = (
            (sprite1.velocity().0 * sprite1.mass() + sprite2.velocity().0 * sprite2.mass())
                / (sprite1.mass() + sprite2.mass()),
            (sprite1.velocity().1 * sprite1.mass() + sprite2.velocity().1 * sprite2.mass())
                / (sprite1.mass() + sprite2.mass()),
        );

        sprite1.set_velocity((
            (1.0 + COEFFICIENT_OF_RESTITUTION) * velocity_com.0
                - COEFFICIENT_OF_RESTITUTION * sprite1.velocity().0,
            (1.0 + COEFFICIENT_OF_RESTITUTION) * velocity_com.1
                - COEFFICIENT_OF_RESTITUTION * sprite1.velocity().1,
        ));

        sprite2.set_velocity((
            (1.0 + COEFFICIENT_OF_RESTITUTION) * velocity_com.0
                - COEFFICIENT_OF_RESTITUTION * sprite2.velocity().0,
            (1.0 + COEFFICIENT_OF_RESTITUTION) * velocity_com.1
                - COEFFICIENT_OF_RESTITUTION * sprite2.velocity().1,
        ));
        return true;
    }

    false
}

trait Body {
    fn position(&self) -> (f32, f32);
    fn set_position(&mut self, position: (f32, f32));
    fn velocity(&self) -> (f32, f32);
    fn set_velocity(&mut self, velocity: (f32, f32));
    fn size(&self) -> (f32, f32);
    fn full_collision_rect(&self) -> Rect {
        let (x, y) = self.position();
        let (w, h) = self.size();
        Rect {
            x: x as u16,
            y: y as u16,
            width: w as u16,
            height: h as u16,
        }
    }
    fn minimal_collision_rect(&self) -> Rect {
        let (x, y) = self.position();
        let (w, h) = self.size();
        Rect {
            x: x as u16,
            y: y as u16,
            width: w as u16,
            height: h as u16,
        }
    }
    fn mass(&self) -> f32;
    fn update(&mut self, _deltatime: f32) {}
    fn image(&self, palette: Palette) -> RgbaImage;
}

#[derive(Clone)]
pub struct Puck {
    position: (f32, f32),
    velocity: (f32, f32),
    possession: Option<GameSide>,
}

impl Puck {
    pub fn new() -> Self {
        // Pick random number from o or 1
        if rand::thread_rng().gen_range(0..=1) == 0 {
            Self {
                position: (79.0, MIN_Y),
                velocity: (0.0, 1.0),
                possession: None,
            }
        } else {
            Self {
                position: (79.0, MAX_Y),
                velocity: (0.0, -1.0),
                possession: None,
            }
        }
    }

    pub fn has_scored(&self) -> Option<GameSide> {
        if self.position.0 <= MIN_X
            && self.position.1 >= GOALIE_AREA_MIN_Y
            && self.position.1 <= GOALIE_AREA_MAX_Y - PUCK_HEIGHT
        {
            return Some(GameSide::Blue);
        }
        if self.position.0 >= MAX_X - PUCK_WIDTH
            && self.position.1 >= GOALIE_AREA_MIN_Y
            && self.position.1 <= GOALIE_AREA_MAX_Y - PUCK_HEIGHT
        {
            return Some(GameSide::Red);
        }
        None
    }

    pub fn attach_to_player(&mut self, player: &Player) {
        let offset = puck_catcher_offset(player.orientation);
        self.set_position((player.position.0 + offset.0, player.position.1 + offset.1));
        self.velocity = player.velocity;
    }

    pub fn can_be_catched_by_player(&self, player: &Player) -> bool {
        // Puck can be catched if it is not in possession of any player
        // and the player catcher pixel overlaps with the puck full collision rect.
        let catcher_position = player.catcher_position();
        let position = Position {
            x: catcher_position.0 as u16,
            y: catcher_position.1 as u16,
        };
        self.possession.is_none()
            && player.after_shooting_counter == 0.0
            && self.full_collision_rect().contains(position)
    }

    pub fn can_be_stolen_by_player(&self, player: &Player) -> bool {
        // Puck can be stolen if it is in possession of the other player
        // and the player catcher pixel overlaps with the puck full collision rect.
        let catcher_position = player.catcher_position();
        let position = Position {
            x: catcher_position.0 as u16,
            y: catcher_position.1 as u16,
        };
        self.possession.is_some()
            && self.possession.unwrap() != player.side
            && player.after_got_stolen_counter == 0.0
            && self.minimal_collision_rect().contains(position)
    }
}

impl Body for Puck {
    fn position(&self) -> (f32, f32) {
        self.position
    }

    fn set_position(&mut self, position: (f32, f32)) {
        let (w1, h1) = self.size();
        let (mut new_x1, mut new_y1) = position;
        if new_x1 < MIN_X {
            new_x1 = MIN_X;
            self.set_velocity((-self.velocity.0, self.velocity.1));
        } else if new_x1 + w1 > MAX_X {
            new_x1 = MAX_X - w1;
            self.set_velocity((-self.velocity.0, self.velocity.1));
        }

        if new_y1 < MIN_Y {
            new_y1 = MIN_Y;
            self.set_velocity((self.velocity.0, -self.velocity.1));
        } else if new_y1 + h1 > MAX_Y {
            new_y1 = MAX_Y - h1;
            self.set_velocity((self.velocity.0, -self.velocity.1));
        }

        self.position = (new_x1, new_y1);
    }

    fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: (f32, f32)) {
        let (mut vx, mut vy) = velocity;
        let speed = (vx.powf(2.0) + vy.powf(2.0)).sqrt();
        if speed > MAX_PUCK_VELOCITY {
            vx = vx * MAX_PUCK_VELOCITY / speed;
            vy = vy * MAX_PUCK_VELOCITY / speed;
        }
        self.velocity = (vx, vy);
    }

    fn size(&self) -> (f32, f32) {
        (PUCK_WIDTH, PUCK_HEIGHT)
    }

    fn mass(&self) -> f32 {
        PUCK_MASS
    }

    fn full_collision_rect(&self) -> Rect {
        // A 6x6 rect around the puck
        let (x, y) = self.position();
        let (w, h) = self.size();
        Rect {
            x: x as u16 - 2,
            y: y as u16 - 2,
            width: w as u16 + 4,
            height: h as u16 + 4,
        }
    }

    fn update(&mut self, deltatime: f32) {
        let (x, y) = self.position();
        let (vx, vy) = self.velocity();
        // Apply friction
        self.set_velocity((
            vx * PUCK_FRICTION_VELOCITY_LOSS,
            vy * PUCK_FRICTION_VELOCITY_LOSS,
        ));
        let (vx, vy) = self.velocity();
        self.set_position((x + vx * deltatime, y + vy * deltatime));
    }

    fn image(&self, palette: Palette) -> RgbaImage {
        match palette {
            Palette::Dark => PUCK_DARK.clone(),
            Palette::Light => PUCK_LIGHT.clone(),
            Palette::Basket => PUCK_DARK.clone(),
            Palette::Alt => PUCK_GOLD.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Player {
    side: GameSide,
    position: (f32, f32),
    velocity: (f32, f32),
    orientation: Orientation,
    new_orientation: Option<Orientation>,
    shooting_direction: Option<(f32, f32)>,
    shooting_counter: f32,
    after_shooting_counter: f32,
    after_got_stolen_counter: f32,
}

impl Player {
    pub fn new(side: GameSide) -> Self {
        let position = match side {
            GameSide::Red => RED_INITIAL_POSITION,
            GameSide::Blue => BLUE_INITIAL_POSITION,
        };
        let orientation = match side {
            GameSide::Red => Orientation::Right,
            GameSide::Blue => Orientation::Left,
        };
        Self {
            side,
            position,
            velocity: (0.0, 0.0),
            orientation,
            new_orientation: None,
            shooting_direction: None,
            shooting_counter: 0.0,
            after_shooting_counter: 0.0,
            after_got_stolen_counter: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.position = match self.side {
            GameSide::Red => RED_INITIAL_POSITION,
            GameSide::Blue => BLUE_INITIAL_POSITION,
        };
        self.velocity = (0.0, 0.0);
        self.orientation = match self.side {
            GameSide::Red => Orientation::Right,
            GameSide::Blue => Orientation::Left,
        };
        self.new_orientation = None;
        self.shooting_direction = None;
        self.shooting_counter = 0.0;
        self.after_shooting_counter = 0.0;
    }

    pub fn catcher_position(&self) -> (f32, f32) {
        let offset = puck_catcher_offset(self.orientation);
        (self.position.0 + offset.0, self.position.1 + offset.1)
    }

    fn head_position_offset(&self) -> (u16, u16) {
        match self.orientation {
            Orientation::Up => (4, 3),
            Orientation::UpLeft => (5, 10),
            Orientation::Left => (3, 13),
            Orientation::DownLeft => (10, 7),
            Orientation::Down => (13, 2),
            Orientation::DownRight => (7, 2),
            Orientation::Right => (2, 4),
            Orientation::UpRight => (2, 5),
        }
    }

    fn rotate(&mut self, new_orientation: Orientation) {
        let previous_head_position = (
            self.position.0 + self.head_position_offset().0 as f32,
            self.position.1 + self.head_position_offset().1 as f32,
        );
        self.orientation = new_orientation;
        self.new_orientation = None;
        // After rotating, realign the player so that the head position did not change
        let new_head_position = (
            self.position.0 + self.head_position_offset().0 as f32,
            self.position.1 + self.head_position_offset().1 as f32,
        );
        let (dx, dy) = (
            previous_head_position.0 - new_head_position.0,
            previous_head_position.1 - new_head_position.1,
        );
        self.position = (self.position.0 + dx, self.position.1 + dy);
    }
}

impl Body for Player {
    fn position(&self) -> (f32, f32) {
        self.position
    }

    fn set_position(&mut self, position: (f32, f32)) {
        let (w1, h1) = self.size();
        let (mut new_x1, mut new_y1) = position;
        if new_x1 < MIN_X {
            new_x1 = MIN_X;
            self.set_velocity((
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.0,
                self.velocity.1,
            ));
        } else if new_x1 + w1 > MAX_X {
            new_x1 = MAX_X - w1;
            self.set_velocity((
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.0,
                self.velocity.1,
            ));
        }

        if new_y1 < MIN_Y {
            new_y1 = MIN_Y;
            self.set_velocity((
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.0,
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.1,
            ));
        } else if new_y1 + h1 > MAX_Y {
            new_y1 = MAX_Y - h1;
            self.set_velocity((
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.0,
                -COFFICIENT_OF_WALL_BOUNCING * self.velocity.1,
            ));
        }

        self.position = (new_x1, new_y1);
    }

    fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: (f32, f32)) {
        let (mut vx, mut vy) = velocity;
        let speed = (vx.powf(2.0) + vy.powf(2.0)).sqrt();
        if speed > MAX_PLAYER_VELOCITY {
            vx = vx * MAX_PLAYER_VELOCITY / speed;
            vy = vy * MAX_PLAYER_VELOCITY / speed;
        }
        self.velocity = (vx, vy);
    }

    fn size(&self) -> (f32, f32) {
        match self.orientation {
            Orientation::Up | Orientation::Down => (20.0, 8.0),
            Orientation::Left | Orientation::Right => (8.0, 20.0),
            _ => (15.0, 15.0),
        }
    }

    fn minimal_collision_rect(&self) -> Rect {
        let (x, y) = self.position();

        match self.orientation {
            Orientation::Up => Rect {
                x: x as u16,
                y: y as u16,
                width: 14,
                height: 8,
            },
            Orientation::UpLeft => Rect {
                x: x as u16,
                y: y as u16 + 5,
                width: 13,
                height: 10,
            },
            Orientation::Left => Rect {
                x: x as u16,
                y: y as u16 + 6,
                width: 8,
                height: 14,
            },
            Orientation::DownLeft => Rect {
                x: x as u16 + 5,
                y: y as u16 + 2,
                width: 10,
                height: 13,
            },
            Orientation::Down => Rect {
                x: x as u16 + 6,
                y: y as u16,
                width: 14,
                height: 8,
            },
            Orientation::DownRight => Rect {
                x: x as u16 + 2,
                y: y as u16,
                width: 13,
                height: 10,
            },
            Orientation::Right => Rect {
                x: x as u16,
                y: y as u16,
                width: 8,
                height: 14,
            },
            Orientation::UpRight => Rect {
                x: x as u16,
                y: y as u16,
                width: 10,
                height: 13,
            },
        }
    }

    fn mass(&self) -> f32 {
        PLAYER_MASS
    }

    fn update(&mut self, deltatime: f32) {
        if let Some(new_orientation) = self.new_orientation {
            self.rotate(new_orientation);
        }

        let (x, y) = self.position();
        let (vx, vy) = self.velocity();
        // Apply friction
        self.set_velocity((
            vx * PLAYER_FRICTION_VELOCITY_LOSS,
            vy * PLAYER_FRICTION_VELOCITY_LOSS,
        ));
        let (vx, vy) = self.velocity();

        self.set_position((x + vx * deltatime, y + vy * deltatime));

        if self.after_shooting_counter > 0.0 {
            self.after_shooting_counter = (self.after_shooting_counter - deltatime).max(0.0);
        }

        if self.after_got_stolen_counter > 0.0 {
            self.after_got_stolen_counter = (self.after_got_stolen_counter - deltatime).max(0.0);
        }
    }

    fn image(&self, _: Palette) -> RgbaImage {
        match self.side {
            GameSide::Red => RED_PLAYER[self.orientation as usize].clone(),
            GameSide::Blue => BLUE_PLAYER[self.orientation as usize].clone(),
        }
    }
}

#[derive(Clone)]
pub struct Goalie {
    side: GameSide,
    position: (f32, f32),
    velocity: (f32, f32),
    saves: usize,
}

impl Goalie {
    pub fn new(side: GameSide) -> Self {
        let position = match side {
            GameSide::Red => (MIN_X, RED_INITIAL_POSITION.1),
            GameSide::Blue => (MAX_X - GOALIE_WIDTH, BLUE_INITIAL_POSITION.1),
        };
        let velocity = (0.0, 0.0);

        Self {
            side,
            position,
            velocity,
            saves: 0,
        }
    }
}

impl Body for Goalie {
    fn position(&self) -> (f32, f32) {
        self.position
    }

    fn set_position(&mut self, position: (f32, f32)) {
        self.position.1 = position.1.min(GOALIE_MAX_Y).max(GOALIE_MIN_Y);
    }

    fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: (f32, f32)) {
        // Goalies only have a vertical velocity
        self.velocity = (0.0, velocity.1);
    }

    fn size(&self) -> (f32, f32) {
        (GOALIE_WIDTH, GOALIE_HEIGHT)
    }

    fn full_collision_rect(&self) -> Rect {
        match self.side {
            GameSide::Red => Rect {
                x: MIN_X as u16,
                y: GOALIE_MIN_Y as u16 - 1,
                width: GOALIE_AREA_WIDTH as u16,
                height: GOALIE_AREA_HEIGHT as u16,
            },
            GameSide::Blue => Rect {
                x: (MAX_X - GOALIE_AREA_WIDTH) as u16,
                y: GOALIE_MIN_Y as u16 - 1,
                width: GOALIE_AREA_WIDTH as u16,
                height: GOALIE_AREA_HEIGHT as u16,
            },
        }
    }

    fn mass(&self) -> f32 {
        GOALIE_MASS
    }

    fn update(&mut self, _deltatime: f32) {}

    fn image(&self, _: Palette) -> RgbaImage {
        match self.side {
            GameSide::Red => RED_GOALIE.clone(),
            GameSide::Blue => BLUE_GOALIE.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Client {
    id: usize,
    terminal: SshTerminal,
    is_connected: bool,
    palette: Palette,
}

impl Client {
    pub fn new(id: usize, terminal: SshTerminal) -> Self {
        Self {
            id,
            terminal,
            is_connected: true,
            palette: Palette::Dark,
        }
    }

    pub fn clear(&mut self) -> AppResult<()> {
        if self.is_connected {
            self.terminal.draw(|f| {
                let mut lines = vec![];
                for _ in 0..f.size().height {
                    lines.push(Line::from(" ".repeat(f.size().width.into())));
                }
                let clear = Paragraph::new(lines).style(Color::White);
                f.render_widget(clear, f.size());
            })?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Game {
    red_client: Client,
    blue_client: Client,
    red_player: Player,
    blue_player: Player,
    red_goalie: Goalie,
    blue_goalie: Goalie,
    red_score: u8,
    blue_score: u8,
    puck: Puck,
    skate_traces: Vec<(f32, f32)>,
    pub id: uuid::Uuid,
    timer: u128,
    last_tick: Instant,
    fps: f32,
    state: GameState,
}

impl Game {
    pub fn new(red_client: (usize, SshTerminal), blue_client: (usize, SshTerminal)) -> Self {
        let mut game = Self {
            red_client: Client::new(red_client.0, red_client.1),
            blue_client: Client::new(blue_client.0, blue_client.1),
            red_player: Player::new(GameSide::Red),
            blue_player: Player::new(GameSide::Blue),
            red_goalie: Goalie::new(GameSide::Red),
            blue_goalie: Goalie::new(GameSide::Blue),
            red_score: 0,
            blue_score: 0,
            puck: Puck::new(),
            skate_traces: vec![],
            id: uuid::Uuid::new_v4(),
            timer: 0,
            last_tick: Instant::now(),
            fps: 0.0,
            state: GameState::Starting {
                time: Instant::now(),
            },
        };

        game.red_client
            .clear()
            .unwrap_or_else(|e| log::error!("Failed to clear red client terminal: {e}"));
        game.blue_client
            .clear()
            .unwrap_or_else(|e| log::error!("Failed to clear red client terminal: {e}"));
        game
    }

    pub fn clear_client(&mut self, client_id: usize) {
        if self.red_client.id == client_id {
            self.red_client
                .clear()
                .unwrap_or_else(|e| log::error!("Failed to clear red client terminal: {e}"));
        } else {
            self.blue_client
                .clear()
                .unwrap_or_else(|e| log::error!("Failed to clear red client terminal: {e}"));
        }
    }

    fn reset(&mut self) {
        self.red_player.reset();
        self.blue_player.reset();
        self.puck = Puck::new();
        self.state = GameState::Starting {
            time: Instant::now(),
        };
        self.skate_traces.clear();
    }

    fn close(&mut self) {
        self.red_client.is_connected = false;
        self.blue_client.is_connected = false;
    }

    pub fn is_over(&self) -> bool {
        !self.red_client.is_connected && !self.blue_client.is_connected
    }

    pub fn is_running(&self) -> bool {
        self.red_client.is_connected && self.blue_client.is_connected
    }

    pub fn client_ids(&self) -> (usize, usize) {
        (self.red_client.id, self.blue_client.id)
    }

    pub fn handle_input(&mut self, client_id: usize, key_code: KeyCode) {
        if key_code == KeyCode::Esc {
            if self.red_client.id == client_id {
                self.red_client.is_connected = false;
            } else {
                self.blue_client.is_connected = false;
            }
            return;
        }

        if key_code == KeyCode::Char('p') {
            if self.red_client.id == client_id {
                self.red_client.palette = self.red_client.palette.next();
            } else {
                self.blue_client.palette = self.blue_client.palette.next();
            }
            return;
        }

        if self.state != GameState::Running {
            return;
        }

        let player = if self.red_client.id == client_id {
            &mut self.red_player
        } else {
            &mut self.blue_player
        };

        if player.shooting_counter > 0.0 {
            let mut shooting_direction = player.shooting_direction.unwrap_or(player.velocity);

            match key_code {
                KeyCode::Up => {
                    shooting_direction = (
                        shooting_direction.0,
                        shooting_direction.1 - SHOOTING_DIRECTION_MODIFIER,
                    );
                }
                KeyCode::Down => {
                    shooting_direction = (
                        shooting_direction.0,
                        shooting_direction.1 + SHOOTING_DIRECTION_MODIFIER,
                    );
                }
                KeyCode::Left => {
                    shooting_direction = (
                        shooting_direction.0 - SHOOTING_DIRECTION_MODIFIER,
                        shooting_direction.1,
                    );
                }
                KeyCode::Right => {
                    shooting_direction = (
                        shooting_direction.0 + SHOOTING_DIRECTION_MODIFIER,
                        shooting_direction.1,
                    );
                }
                _ => {}
            }
            player.shooting_direction = Some(shooting_direction);
        } else {
            // Shooting
            if key_code == KeyCode::Char(' ') && player.after_shooting_counter == 0.0 {
                if let Some(side) = self.puck.possession {
                    if side == player.side {
                        player.shooting_counter = SHOOTING_COUNTER_MILLISECONDS;
                        player.velocity.0 *= 0.85;
                        player.velocity.1 *= 0.85;
                        self.puck.velocity.0 *= 0.85;
                        self.puck.velocity.1 *= 0.85;
                        player.new_orientation = Some(player.orientation.previous());
                        // Set shooting direction to the current orientation
                        // offset by 1 so that we shoot in the movement direction
                        player.shooting_direction = match player.orientation {
                            Orientation::Up => Some((1.0, -1.0).normalize()),
                            Orientation::UpLeft => Some((0.0, -1.0).normalize()),
                            Orientation::Left => Some((-1.0, -1.0).normalize()),
                            Orientation::DownLeft => Some((-1.0, 0.0).normalize()),
                            Orientation::Down => Some((-1.0, 1.0).normalize()),
                            Orientation::DownRight => Some((0.0, 1.0).normalize()),
                            Orientation::Right => Some((1.0, 1.0).normalize()),
                            Orientation::UpRight => Some((1.0, 0.0).normalize()),
                        };
                    }
                }
            } else {
                // Movement
                let current_speed = player.velocity.magnitude();

                let natural_orientation = match key_code {
                    KeyCode::Up => {
                        if player.velocity.1 > 0.0 {
                            player.velocity.1 -= DECELERATION;
                        } else {
                            player.velocity.1 -= ACCELERATION;
                        }
                        Orientation::UpLeft
                    }
                    KeyCode::Down => {
                        if player.velocity.1 < 0.0 {
                            player.velocity.1 += DECELERATION;
                        } else {
                            player.velocity.1 += ACCELERATION;
                        }
                        Orientation::DownRight
                    }
                    KeyCode::Left => {
                        if player.velocity.0 > 0.0 {
                            player.velocity.0 -= DECELERATION;
                        } else {
                            player.velocity.0 -= ACCELERATION;
                        }
                        Orientation::DownLeft
                    }
                    KeyCode::Right => {
                        if player.velocity.0 < 0.0 {
                            player.velocity.0 += DECELERATION;
                        } else {
                            player.velocity.0 += ACCELERATION;
                        }
                        Orientation::UpRight
                    }
                    _ => player.orientation,
                };

                // If player current orientation is not the natural orientation,
                // try to align one step at the time
                if current_speed > 0.0 && player.orientation != natural_orientation {
                    let diff = (natural_orientation as isize - player.orientation as isize + 8) % 8;
                    if diff > 4 {
                        player.new_orientation = Some(player.orientation.previous());
                    } else {
                        player.new_orientation = Some(player.orientation.next());
                    }
                }
            }
        }
    }

    pub fn update(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let deltatime = now.duration_since(self.last_tick).as_millis() as f32;
        if deltatime < MINIMUM_DELTATIME_MILLISECONDS {
            return Ok(());
        }

        match self.state {
            GameState::Starting { time } => {
                if now.duration_since(time).as_millis() >= STARTING_DELAY_MILLISECONDS {
                    self.state = GameState::Running;
                }
            }
            GameState::Running => {
                self.update_running(deltatime)?;
                self.timer += deltatime as u128;
                if self.timer > GAME_DURATION_MILLISECONDS {
                    self.state = GameState::Ending {
                        time: Instant::now(),
                    };
                }
            }
            GameState::AfterGoal { time, scored: _ } => {
                if now.duration_since(time).as_millis() >= AFTER_GOAL_DELAY_MILLISECONDS {
                    self.reset();
                }
            }
            GameState::Ending { time } => {
                if now.duration_since(time).as_millis() >= ENDING_DELAY_MILLISECONDS {
                    self.close();
                }
            }
        }
        self.fps = 1000.0 / deltatime;
        self.last_tick = now;

        Ok(())
    }

    fn update_running(&mut self, deltatime: f32) -> AppResult<()> {
        let red_previous_position = self.red_player.position;
        let red_previous_orientation = self.red_player.orientation;
        let blue_previous_position = self.blue_player.position;
        let blue_previous_orientation = self.blue_player.orientation;

        let normalized_deltatime = deltatime / MINIMUM_DELTATIME_MILLISECONDS;

        self.red_player.update(normalized_deltatime);
        let red_goalie_head_position_y =
            self.red_player.position.1 + self.red_player.head_position_offset().1 as f32 - 2.0; // -2 is the goalie head offset.
        self.red_goalie
            .set_position((MIN_X, red_goalie_head_position_y));
        self.red_goalie.set_velocity(self.red_player.velocity);

        self.blue_player.update(normalized_deltatime);
        let blue_goalie_head_position_y =
            self.blue_player.position.1 + self.blue_player.head_position_offset().1 as f32 - 2.0; // -2 is the goalie head offset.
        self.blue_goalie
            .set_position((MAX_X - GOALIE_WIDTH, blue_goalie_head_position_y));
        self.blue_goalie.set_velocity(self.blue_player.velocity);
        self.puck.update(normalized_deltatime);

        // Check collisions between players
        if resolve_collision(
            &mut self.red_player,
            &mut self.blue_player,
            CollisionType::Minimal,
            CollisionType::Minimal,
        ) {
            self.red_player.rotate(red_previous_orientation);
            self.red_player.set_position(red_previous_position);
            self.blue_player.rotate(blue_previous_orientation);
            self.blue_player.set_position(blue_previous_position);
        }

        // Check collision between red and goalies.
        if resolve_collision(
            &mut self.red_player,
            &mut self.red_goalie,
            CollisionType::Minimal,
            CollisionType::Full,
        ) || resolve_collision(
            &mut self.red_player,
            &mut self.red_goalie,
            CollisionType::Full,
            CollisionType::Minimal,
        ) || resolve_collision(
            &mut self.red_player,
            &mut self.blue_goalie,
            CollisionType::Minimal,
            CollisionType::Full,
        ) || resolve_collision(
            &mut self.red_player,
            &mut self.blue_goalie,
            CollisionType::Full,
            CollisionType::Minimal,
        ) {
            self.red_player.rotate(red_previous_orientation);
            self.red_player.set_position(red_previous_position);
            self.red_player.set_velocity((0.0, 0.0));
        }

        // Check collision between blue and goalies
        if resolve_collision(
            &mut self.blue_player,
            &mut self.blue_goalie,
            CollisionType::Minimal,
            CollisionType::Full,
        ) || resolve_collision(
            &mut self.blue_player,
            &mut self.blue_goalie,
            CollisionType::Full,
            CollisionType::Minimal,
        ) || resolve_collision(
            &mut self.blue_player,
            &mut self.red_goalie,
            CollisionType::Minimal,
            CollisionType::Full,
        ) || resolve_collision(
            &mut self.blue_player,
            &mut self.red_goalie,
            CollisionType::Full,
            CollisionType::Minimal,
        ) {
            self.blue_player.rotate(blue_previous_orientation);
            self.blue_player.set_position(blue_previous_position);
            self.blue_player.set_velocity((0.0, 0.0));
        }

        if self.red_player.position != red_previous_position {
            let head_position = (
                self.red_player.position.0 + self.red_player.head_position_offset().0 as f32,
                self.red_player.position.1 + self.red_player.head_position_offset().1 as f32,
            );
            self.skate_traces.push(head_position);
        }
        if self.blue_player.position != blue_previous_position {
            let head_position = (
                self.blue_player.position.0 + self.blue_player.head_position_offset().0 as f32,
                self.blue_player.position.1 + self.blue_player.head_position_offset().1 as f32,
            );
            self.skate_traces.push(head_position);
        }

        while self.skate_traces.len() > SKATE_TRACE_LENGTH {
            self.skate_traces.remove(0);
        }

        let puck_previous_position = self.puck.position;
        // Check collision between puck and goalies
        // FIXME: sometimes puck is tucked inside goalie
        if resolve_collision(
            &mut self.puck,
            &mut self.red_goalie,
            CollisionType::Minimal,
            CollisionType::Minimal,
        ) {
            self.puck.set_position(puck_previous_position);
            self.red_goalie.saves += 1;
        } else if resolve_collision(
            &mut self.puck,
            &mut self.blue_goalie,
            CollisionType::Minimal,
            CollisionType::Minimal,
        ) {
            self.puck.set_position(puck_previous_position);
            self.blue_goalie.saves += 1;
        }

        // Check collision between puck and players
        if resolve_collision(
            &mut self.puck,
            &mut self.red_player,
            CollisionType::Minimal,
            CollisionType::Minimal,
        ) || resolve_collision(
            &mut self.puck,
            &mut self.blue_player,
            CollisionType::Minimal,
            CollisionType::Minimal,
        ) {
            self.puck.set_position(puck_previous_position);
        }

        // Check for goals!
        match self.puck.has_scored() {
            Some(GameSide::Red) => {
                self.red_score += 1;
                self.state = GameState::AfterGoal {
                    time: Instant::now(),
                    scored: GameSide::Red,
                };
                return Ok(());
            }
            Some(GameSide::Blue) => {
                self.blue_score += 1;
                self.state = GameState::AfterGoal {
                    time: Instant::now(),
                    scored: GameSide::Blue,
                };
                return Ok(());
            }
            None => {}
        }

        // Logic related to puck possession
        match self.puck.possession {
            Some(GameSide::Red) => {
                if self.puck.can_be_stolen_by_player(&self.blue_player) {
                    self.puck.possession = Some(GameSide::Blue);
                    self.red_player.after_got_stolen_counter =
                        AFTER_GOT_STOLEN_COUNTER_MILLISECONDS;
                }
            }
            Some(GameSide::Blue) => {
                if self.puck.can_be_stolen_by_player(&self.red_player) {
                    self.puck.possession = Some(GameSide::Red);
                    self.blue_player.after_got_stolen_counter =
                        AFTER_GOT_STOLEN_COUNTER_MILLISECONDS;
                }
            }
            None => {
                match (
                    self.puck.can_be_catched_by_player(&self.red_player),
                    self.puck.can_be_catched_by_player(&self.blue_player),
                ) {
                    (true, true) => {
                        // Puck goes to the fastest moving player
                        if self.red_player.velocity.magnitude()
                            > self.blue_player.velocity.magnitude()
                        {
                            self.puck.possession = Some(GameSide::Red);
                        } else if self.blue_player.velocity.magnitude()
                            > self.red_player.velocity.magnitude()
                        {
                            self.puck.possession = Some(GameSide::Blue);
                        } else {
                            // do nothing
                        }
                    }
                    (true, false) => {
                        self.puck.possession = Some(GameSide::Red);
                    }
                    (false, true) => {
                        self.puck.possession = Some(GameSide::Blue);
                    }
                    (false, false) => {
                        self.puck.possession = None;
                    }
                }
            }
        }

        // Puck positioning logic.
        // If the puck is in possession, it follows the player unless the player is shooting.
        if let Some(side) = self.puck.possession {
            let (player, other) = if side == GameSide::Red {
                (&mut self.red_player, &mut self.blue_player)
            } else {
                (&mut self.blue_player, &mut self.red_player)
            };

            if player.shooting_counter > 0.0 {
                player.shooting_counter -= deltatime;
                // If the player is shooting counter went to 0, the puck follows the shooting direction.
                if player.shooting_counter <= 0.0 {
                    player.shooting_counter = 0.0;
                    player.after_shooting_counter = AFTER_SHOOTING_COUNTER_MILLISECONDS;
                    player.new_orientation = Some(((player.orientation as u8 + 1) % 8).into());
                    self.puck.possession = None;

                    // FIXME: put shooting direction and counter together in a single variable
                    self.puck.velocity = player
                        .shooting_direction
                        .unwrap_or(player.velocity)
                        .mul(SHOOTING_POWER);

                    player.shooting_direction = None;
                }
            } else {
                self.puck.attach_to_player(&player);
            }

            if other.shooting_counter > 0.0 {
                other.shooting_counter = 0.0;
                other.shooting_direction = None;
            }
        }

        Ok(())
    }

    pub fn draw(&mut self) -> AppResult<()> {
        let timer = if self.timer > GAME_DURATION_MILLISECONDS {
            0
        } else {
            (GAME_DURATION_MILLISECONDS - self.timer) / 1000
        };

        if self.red_client.is_connected {
            if self
                .red_client
                .terminal
                .draw(|f| {
                    Self::render(
                        f,
                        self.red_client.palette,
                        &self.red_player,
                        &self.red_goalie,
                        &self.blue_player,
                        &self.blue_goalie,
                        &self.puck,
                        &self.skate_traces,
                        self.red_score,
                        self.blue_score,
                        self.red_goalie.saves,
                        self.blue_goalie.saves,
                        timer,
                        self.fps,
                        self.state,
                        GameSide::Red,
                    )
                    .unwrap_or_else(|e| {
                        log::error!("Failed to draw game: {}", e);
                    })
                })
                .is_err()
            {
                self.red_client.is_connected = false;
            }
        }
        if self.blue_client.is_connected {
            if self
                .blue_client
                .terminal
                .draw(|f| {
                    Self::render(
                        f,
                        self.blue_client.palette,
                        &self.red_player,
                        &self.red_goalie,
                        &self.blue_player,
                        &self.blue_goalie,
                        &self.puck,
                        &self.skate_traces,
                        self.red_score,
                        self.blue_score,
                        self.red_goalie.saves,
                        self.blue_goalie.saves,
                        timer,
                        self.fps,
                        self.state,
                        GameSide::Blue,
                    )
                    .unwrap_or_else(|e| {
                        log::error!("Failed to draw game: {}", e);
                    })
                })
                .is_err()
            {
                self.blue_client.is_connected = false;
            }
        }

        Ok(())
    }

    fn render(
        frame: &mut Frame,
        palette: Palette,
        red_player: &impl Body,
        red_goalie: &impl Body,
        blue_player: &impl Body,
        blue_goalie: &impl Body,
        puck: &impl Body,
        skate_traces: &[(f32, f32)],
        red_score: u8,
        blue_score: u8,
        red_saves: usize,
        blue_saves: usize,
        timer: u128,
        fps: f32,
        state: GameState,
        rules_side: GameSide,
    ) -> AppResult<()> {
        let split =
            Layout::vertical([Constraint::Length(7), Constraint::Min(1)]).split(frame.size());

        let mut img = base_image(palette);

        for (x, y) in skate_traces {
            img.put_pixel(*x as u32, *y as u32, skate_trace_color(palette));
        }

        img.copy_non_trasparent_from(
            &red_player.image(palette),
            red_player.position().0 as u32,
            red_player.position().1 as u32,
        )?;

        img.copy_non_trasparent_from(
            &red_goalie.image(palette),
            red_goalie.position().0 as u32,
            red_goalie.position().1 as u32,
        )?;

        img.copy_non_trasparent_from(
            &blue_player.image(palette),
            blue_player.position().0 as u32,
            blue_player.position().1 as u32,
        )?;
        img.copy_non_trasparent_from(
            &blue_goalie.image(palette),
            blue_goalie.position().0 as u32,
            blue_goalie.position().1 as u32,
        )?;

        img.copy_non_trasparent_from(
            &puck.image(palette),
            puck.position().0 as u32,
            puck.position().1 as u32,
        )?;

        let paragraph = Paragraph::new(img_to_lines(&img));
        frame.render_widget(paragraph, split[1]);

        let info_rect = Rect::new(frame.size().width - 20, frame.size().height - 1, 10, 1);
        frame.render_widget(Paragraph::new(format!("FPS:{}", fps as u32)), info_rect);

        let top_split = Layout::horizontal([
            Constraint::Length(20),
            Constraint::Length(43),
            Constraint::Length(34),
            Constraint::Length(43),
            Constraint::Length(20),
        ])
        .split(Rect {
            x: 0,
            y: 1,
            width: frame.size().width,
            height: 6,
        });

        let red_score_paragraph = red_score.big_font_styled(Color::Red, Color::Yellow);

        let horizontal = if red_score < 10 { 5 } else { 1 };
        let area = top_split[0].inner(&Margin {
            horizontal,
            vertical: 0,
        });
        frame.render_widget(red_score_paragraph, area);

        match rules_side {
            GameSide::Red => {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(format!("Saves {}", red_saves)),
                        Line::from("← ↑ → ↓: move"),
                        Line::from("space: shoot"),
                        Line::from("p: change palette"),
                        Line::from("Esc: close game"),
                    ])
                    .centered(),
                    top_split[1],
                );
                frame.render_widget(
                    Paragraph::new(format!("Saves {}", blue_saves)).centered(),
                    top_split[3],
                );
            }
            GameSide::Blue => {
                frame.render_widget(
                    Paragraph::new(format!("Saves {}", red_saves)).centered(),
                    top_split[1],
                );
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(format!("Saves {}", blue_saves)),
                        Line::from("← ↑ → ↓: move"),
                        Line::from("space: shoot"),
                        Line::from("p: change palette"),
                        Line::from("Esc: close game"),
                    ])
                    .centered(),
                    top_split[3],
                );
            }
        }

        let blue_score_paragraph = blue_score.big_font_styled(Color::Blue, Color::LightMagenta);
        let horizontal = if blue_score < 10 { 5 } else { 1 };
        let area = top_split[4].inner(&Margin {
            horizontal,
            vertical: 0,
        });
        frame.render_widget(blue_score_paragraph, area);

        let timer_split = Layout::horizontal([
            Constraint::Length(10),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .split(top_split[2]);

        let (color_1, color_2) = match palette {
            Palette::Dark => (Color::Cyan, Color::White),
            Palette::Light => (Color::DarkGray, Color::Black),
            Palette::Basket => (Color::Magenta, Color::LightMagenta),
            Palette::Alt => (Color::Green, Color::Red),
        };

        let minutes_paragraph = ((timer / 60) as u8).big_font_styled(color_1, color_2);
        let seconds_tens_paragraph = (((timer % 60) / 10) as u8).big_font_styled(color_1, color_2);
        let seconds_units_paragraph = (((timer % 60) % 10) as u8).big_font_styled(color_1, color_2);

        frame.render_widget(minutes_paragraph, timer_split[0]);
        frame.render_widget(dots(color_1, color_2), timer_split[1]);
        frame.render_widget(seconds_tens_paragraph, timer_split[2]);
        frame.render_widget(seconds_units_paragraph, timer_split[3]);

        match state {
            GameState::Starting { time } => {
                let rect = Rect::new(
                    (MIN_X + MAX_X) as u16 / 2 - 5,
                    (MIN_Y + MAX_Y) as u16 / 4 + 5,
                    10,
                    10,
                );
                let elapsed = time.elapsed().as_millis();
                let countdown_paragraph = if STARTING_DELAY_MILLISECONDS > elapsed {
                    (((STARTING_DELAY_MILLISECONDS - elapsed) / 1000) as u8 + 1)
                        .big_font_styled(color_1, color_2)
                } else {
                    Paragraph::new("")
                };

                frame.render_widget(countdown_paragraph, rect);
            }
            GameState::AfterGoal { time: _, scored } => {
                let rect = Rect::new(
                    (MIN_X + MAX_X) as u16 / 2 - 44,
                    (MIN_Y + MAX_Y) as u16 / 4 + 5,
                    88,
                    10,
                );
                let scored = if scored == GameSide::Red {
                    red_scored(color_1, color_2)
                } else {
                    blue_scored(color_1, color_2)
                };
                frame.render_widget(scored, rect);
            }
            GameState::Ending { .. } => {
                let rect = Rect::new(
                    (MIN_X + MAX_X) as u16 / 2 - 36,
                    (MIN_Y + MAX_Y) as u16 / 4 + 5,
                    72,
                    10,
                );
                let congrats = if red_score > blue_score {
                    red_won(color_1, color_2)
                } else if blue_score > red_score {
                    blue_won(color_1, color_2)
                } else {
                    draw(color_1, color_2)
                };
                frame.render_widget(congrats, rect);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn connections_state(&self) -> (bool, bool) {
        (self.red_client.is_connected, self.blue_client.is_connected)
    }
}

#[cfg(test)]

mod test {
    use super::*;
    use core::time;
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    #[test]
    fn test_puck_position() {
        let mut player = Player::new(GameSide::Red);
        player.set_position((50.0, 40.0));
        let mut puck = Puck::new();

        let offset = puck_catcher_offset(player.orientation);
        puck.set_position((player.position.0 + offset.0, player.position.1 + offset.1));

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();

        let palette = Palette::Dark;

        for _ in 0..16 {
            let offset = puck_catcher_offset(player.orientation);
            puck.set_position((player.position.0 + offset.0, player.position.1 + offset.1));
            terminal
                .draw(|frame| {
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &player.image(palette),
                        player.position().0 as u32,
                        player.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &puck.image(palette),
                        puck.position().0 as u32,
                        puck.position().1 as u32,
                    )
                    .unwrap();

                    let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                        .split(frame.size());

                    let info = Paragraph::new(format!("Orientation {}", player.orientation as u8));
                    frame.render_widget(info, split[0]);

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, split[1]);
                })
                .unwrap();
            player.rotate(player.orientation.next());
            std::thread::sleep(time::Duration::from_millis(500));
        }
    }

    #[test]
    fn test_player_collision_boxes() {
        let mut full_box_player = Player::new(GameSide::Red);
        full_box_player.set_position((50.0, 40.0));

        let mut minimal_box_player = Player::new(GameSide::Blue);
        minimal_box_player.set_position((100.0, 40.0));

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        for _ in 0..16 {
            terminal
                .draw(|frame| {
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &full_box_player.image(palette),
                        full_box_player.position().0 as u32,
                        full_box_player.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &minimal_box_player.image(palette),
                        minimal_box_player.position().0 as u32,
                        minimal_box_player.position().1 as u32,
                    )
                    .unwrap();

                    // Color in white the border of the collision boxes
                    let full_box = full_box_player.full_collision_rect();

                    for x in full_box.x..full_box.x + full_box.width {
                        img.put_pixel(
                            x as u32,
                            full_box.y as u32 - 1,
                            image::Rgba([255, 255, 255, 255]),
                        );
                        img.put_pixel(
                            x as u32,
                            full_box.y as u32 + full_box.height as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                    }

                    for y in full_box.y..full_box.y + full_box.height {
                        img.put_pixel(
                            full_box.x as u32 - 1,
                            y as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                        img.put_pixel(
                            full_box.x as u32 + full_box.width as u32,
                            y as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                    }
                    let minimal_box = minimal_box_player.minimal_collision_rect();

                    for x in minimal_box.x..minimal_box.x + minimal_box.width {
                        img.put_pixel(
                            x as u32,
                            minimal_box.y as u32 - 1,
                            image::Rgba([255, 255, 255, 255]),
                        );
                        img.put_pixel(
                            x as u32,
                            minimal_box.y as u32 + minimal_box.height as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                    }

                    for y in minimal_box.y..minimal_box.y + minimal_box.height {
                        img.put_pixel(
                            minimal_box.x as u32 - 1,
                            y as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                        img.put_pixel(
                            minimal_box.x as u32 + minimal_box.width as u32,
                            y as u32,
                            image::Rgba([255, 255, 255, 255]),
                        );
                    }

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, frame.size());
                })
                .unwrap();
            full_box_player.rotate(full_box_player.orientation.next());
            minimal_box_player.rotate(minimal_box_player.orientation.previous());

            std::thread::sleep(time::Duration::from_millis(500));
        }
    }

    #[test]
    fn test_player_rotation_center() {
        let mut player = Player::new(GameSide::Red);
        player.set_position((50.0, 40.0));

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        for _ in 0..16 {
            terminal
                .draw(|frame| {
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &player.image(palette),
                        player.position().0 as u32,
                        player.position().1 as u32,
                    )
                    .unwrap();

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, frame.size());
                })
                .unwrap();
            let new_orientation = player.orientation.next();
            player.rotate(new_orientation);
            std::thread::sleep(time::Duration::from_millis(500));
        }
    }

    #[test]
    fn test_goalie_collision_boxes() {
        let mut red_goalie = Goalie::new(GameSide::Red);
        let mut blue_goalie = Goalie::new(GameSide::Blue);

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        for idx in 0..40 {
            let dy = if idx < 20 { 1.0 } else { -1.0 };
            red_goalie.set_position((red_goalie.position.0, red_goalie.position.1 + dy));
            blue_goalie.set_position((blue_goalie.position.0, blue_goalie.position.1 + dy));

            terminal
                .draw(|frame| {
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &red_goalie.image(palette),
                        red_goalie.position().0 as u32,
                        red_goalie.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &blue_goalie.image(palette),
                        blue_goalie.position().0 as u32,
                        blue_goalie.position().1 as u32,
                    )
                    .unwrap();

                    for goalie in [&red_goalie, &blue_goalie].iter() {
                        let full_box = goalie.full_collision_rect();

                        for x in full_box.x..full_box.x + full_box.width {
                            img.put_pixel(
                                x as u32,
                                full_box.y as u32 - 1,
                                image::Rgba([255, 255, 0, 255]),
                            );
                            img.put_pixel(
                                x as u32,
                                full_box.y as u32 + full_box.height as u32,
                                image::Rgba([255, 255, 0, 255]),
                            );
                        }

                        for y in full_box.y..full_box.y + full_box.height {
                            img.put_pixel(
                                full_box.x as u32 - 1,
                                y as u32,
                                image::Rgba([255, 255, 0, 255]),
                            );
                            img.put_pixel(
                                full_box.x as u32 + full_box.width as u32,
                                y as u32,
                                image::Rgba([255, 255, 0, 255]),
                            );
                        }
                        let minimal_box = goalie.minimal_collision_rect();

                        for x in minimal_box.x..minimal_box.x + minimal_box.width {
                            img.put_pixel(
                                x as u32,
                                minimal_box.y as u32 - 1,
                                image::Rgba([0, 0, 255, 255]),
                            );
                            img.put_pixel(
                                x as u32,
                                minimal_box.y as u32 + minimal_box.height as u32,
                                image::Rgba([0, 0, 255, 255]),
                            );
                        }

                        for y in minimal_box.y..minimal_box.y + minimal_box.height {
                            img.put_pixel(
                                minimal_box.x as u32 - 1,
                                y as u32,
                                image::Rgba([0, 0, 255, 255]),
                            );
                            img.put_pixel(
                                minimal_box.x as u32 + minimal_box.width as u32,
                                y as u32,
                                image::Rgba([0, 0, 255, 255]),
                            );
                        }
                    }

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, frame.size());
                })
                .unwrap();

            std::thread::sleep(time::Duration::from_millis(200));
        }
    }

    #[test]
    fn test_goal_areas() {
        let mut puck = Puck::new();
        puck.set_position((MAX_X as f32 - 20.0, 30.0));
        puck.set_velocity((0.02, 0.0));
        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        let mut last_tick = Instant::now();

        let mut score = 0;
        let mut y = 0.0;
        loop {
            let now = Instant::now();
            let deltatime = now.duration_since(last_tick).as_millis() as f32;

            puck.update(deltatime);

            terminal
                .draw(|frame| {
                    let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                        .split(frame.size());
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &puck.image(palette),
                        puck.position().0 as u32,
                        puck.position().1 as u32,
                    )
                    .unwrap();

                    for y in GOALIE_AREA_MIN_Y as u32..=(GOALIE_AREA_MAX_Y - PUCK_HEIGHT) as u32 {
                        img.put_pixel(
                            (MAX_X - PUCK_WIDTH) as u32,
                            y,
                            image::Rgba([255, 255, 0, 255]),
                        );
                        img.put_pixel(MIN_X as u32, y, image::Rgba([255, 255, 0, 255]));
                    }

                    let info = format!("Score {}", score);
                    let paragraph = Paragraph::new(info);
                    frame.render_widget(paragraph, split[0]);

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, split[1]);
                })
                .unwrap();

            if puck.has_scored().is_some() {
                score += 1;
                y += 1.0;
                puck.set_position((MAX_X as f32 - 20.0, 30.0 + y));
                puck.set_velocity((0.025, 0.0));
            } else if puck.velocity.0 < 0.0 {
                y += 1.0;
                puck.set_position((MAX_X as f32 - 20.0, 30.0 + y));
                puck.set_velocity((0.025, 0.0));
            }

            if y > 30.0 {
                break;
            }

            std::thread::sleep(time::Duration::from_millis(20));
            last_tick = now;
        }
    }

    #[test]
    fn test_puck_possession() {
        let mut red_player = Player::new(GameSide::Red);
        red_player.set_position((50.0, 40.0));
        let mut blue_player = Player::new(GameSide::Blue);
        blue_player.set_position((100.0, 40.0));
        let mut puck = Puck::new();

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        puck.possession = Some(GameSide::Red);
        puck.attach_to_player(&red_player);

        for _ in 0..16 {
            terminal
                .draw(|frame| {
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &red_player.image(palette),
                        red_player.position().0 as u32,
                        red_player.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &blue_player.image(palette),
                        blue_player.position().0 as u32,
                        blue_player.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &puck.image(palette),
                        puck.position().0 as u32,
                        puck.position().1 as u32,
                    )
                    .unwrap();

                    //Color in red all pixels within puck full collision rect
                    let full_box = puck.full_collision_rect();
                    for x in full_box.x..full_box.x + full_box.width {
                        for y in full_box.y..full_box.y + full_box.height {
                            img.put_pixel(x as u32, y as u32, image::Rgba([255, 0, 0, 255]));
                        }
                    }

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, frame.size());
                })
                .unwrap();
            red_player.rotate(red_player.orientation.next());
            puck.attach_to_player(&red_player);
            std::thread::sleep(time::Duration::from_millis(500));
        }
    }

    #[test]
    fn test_goalie_collision() {
        let mut puck = Puck::new();
        puck.set_position((MAX_X as f32 - 25.0, 42.0));
        puck.set_velocity((0.85, 0.0));

        let mut goalie = Goalie::new(GameSide::Blue);

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear().unwrap();
        let palette = Palette::Dark;

        let mut last_tick = Instant::now();

        let start = Instant::now();

        loop {
            let now = Instant::now();
            let deltatime = now.duration_since(last_tick).as_millis() as f32;
            let puck_previous_position = puck.position;
            puck.update(deltatime);

            if resolve_collision(
                &mut puck,
                &mut goalie,
                CollisionType::Minimal,
                CollisionType::Minimal,
            ) {
                puck.set_position(puck_previous_position);
            }

            terminal
                .draw(|frame| {
                    let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                        .split(frame.size());
                    let mut img = base_image(palette);

                    img.copy_non_trasparent_from(
                        &goalie.image(palette),
                        goalie.position().0 as u32,
                        goalie.position().1 as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &puck.image(palette),
                        puck.position().0 as u32,
                        puck.position().1 as u32,
                    )
                    .unwrap();

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, split[1]);
                })
                .unwrap();

            std::thread::sleep(time::Duration::from_millis(20));
            last_tick = now;

            if start.elapsed() > time::Duration::from_secs(5) {
                break;
            }
        }
    }
}
