use super::player::Player;
use crate::{
    constants::*,
    traits::{Body, Entity, HitBox, Sprite},
    types::{GameSide, Palette},
    utils::GOALIE_IMAGE_DATA,
};
use glam::{U16Vec2, Vec2};
use image::RgbaImage;

#[derive(Debug, Default)]
pub struct Goalie {
    previous_position: Vec2,
    position: Vec2,
    velocity: Vec2,
    side: GameSide,
    pub saves: usize,
}

impl Goalie {
    pub fn new(side: GameSide) -> Self {
        let mut g = Self {
            side,
            previous_position: Vec2::ZERO,
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            saves: 0,
        };

        g.position = match side {
            GameSide::Red => Vec2::new(MIN_X.into(), RED_INITIAL_POSITION.y),
            GameSide::Blue => Vec2::new((MAX_X - g.size().x).into(), BLUE_INITIAL_POSITION.y),
        };
        g.previous_position = g.position;

        g
    }

    pub fn align_to_player(&mut self, player: &Player) {
        let offset = player.head_position_offset();
        self.set_position(player.position() + offset - U16Vec2::new(0, 2));
    }
}

impl Body for Goalie {
    fn mass(&self) -> f32 {
        f32::INFINITY
    }

    fn previous_position(&self) -> U16Vec2 {
        self.previous_position.as_u16vec2()
    }

    fn position(&self) -> U16Vec2 {
        self.position.as_u16vec2()
    }

    fn set_position(&mut self, position: U16Vec2) {
        let inner_area = match self.side {
            GameSide::Red => RED_AREA_INNER_RECT,
            GameSide::Blue => BLUE_AREA_INNER_RECT,
        };

        let min_y = inner_area.y;
        let max_y = inner_area.y + inner_area.height - self.size().y;
        self.position.y = position.y.max(min_y).min(max_y) as f32;
    }

    fn velocity(&self) -> Vec2 {
        self.velocity
    }

    fn set_velocity(&mut self, _velocity: Vec2) {
        // Goalies body is updated by the player position
    }

    fn update_body(&mut self, _deltatime: f32) {
        // Goalies body is updated by the player position
    }
}

impl Sprite for Goalie {
    fn image(&self, _palette: Palette) -> &RgbaImage {
        &GOALIE_IMAGE_DATA
            .get(&self.side)
            .expect("There should be goalie data")
            .images[0]
    }

    fn hit_box(&self) -> &HitBox {
        &GOALIE_IMAGE_DATA
            .get(&self.side)
            .expect("There should be goalie data")
            .hit_boxes[0]
    }
}

impl Entity for Goalie {}
