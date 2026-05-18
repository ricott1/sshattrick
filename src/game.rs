use super::engine::{goalie::Goalie, player::Player, puck::Puck};
use crate::{
    collision_detection::{are_colliding, inelastic_collision},
    constants::*,
    engine::{area::Area, utils::RectSide},
    traits::{Body, ColliderType, Entity, Sprite},
    types::*,
    utils::*,
};
use crossterm::event::KeyCode;
use glam::{U16Vec2, Vec2};
use image::RgbaImage;
use std::{
    fmt::Display,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum GameState {
    // TODO: add character selection with different stats
    #[default]
    WaitingForPlayer,
    Starting {
        time: Instant,
    },
    Running,
    AfterGoal {
        time: Instant,
        scored: GameSide,
    },
    Ending {
        time: Instant,
    },
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WaitingForPlayer => write!(f, "WaitingForPlayer"),
            Self::Starting { .. } => write!(f, "Starting"),
            Self::Running => write!(f, "Running"),
            Self::AfterGoal { .. } => write!(f, "AfterGoal"),
            Self::Ending { .. } => write!(f, "Ending"),
        }
    }
}

#[derive(Debug, Default)]
pub struct GameData {
    _side: GameSide,
    pub player: Player,
    pub goalie: Goalie,
    pub area: Area,
    pub score: u8,
}

impl GameData {
    pub fn new(side: GameSide) -> Self {
        Self {
            _side: side,
            player: Player::new(side),
            goalie: Goalie::new(side),
            area: Area::new(side),
            score: 0,
        }
    }

    pub fn reset(&mut self) {
        self.player.reset();
    }

    pub fn handle_key_events(&mut self, puck: &mut Puck, key_code: KeyCode) {
        let player = &mut self.player;
        if player.shooting_state.is_shooting() {
            let shooting_modifier = match key_code {
                KeyCode::Up => Vec2::NEG_Y * SHOOTING_DIRECTION_MODIFIER,
                KeyCode::Down => Vec2::Y * SHOOTING_DIRECTION_MODIFIER,
                KeyCode::Left => Vec2::NEG_X * SHOOTING_DIRECTION_MODIFIER,
                KeyCode::Right => Vec2::X * SHOOTING_DIRECTION_MODIFIER,
                _ => Vec2::ZERO,
            };

            player.shooting_state.direction = Some(
                player.shooting_state.direction.unwrap_or(player.velocity) + shooting_modifier,
            );
            return;
        }
        // Shooting
        if key_code == KeyCode::Char(' ') && player.after_shooting_counter == 0.0 {
            if puck.possession == Some(player.side) {
                player.velocity *= 0.85;
                puck.velocity *= 0.85;
                player.new_orientation = Some(player.orientation.previous());
                // Set shooting direction to the current orientation
                // offset by 1 so that we shoot in the movement direction
                let shooting_direction = match player.orientation {
                    Orientation::Up => Vec2::new(1.0, -1.0).normalize(),
                    Orientation::UpLeft => Vec2::new(0.0, -1.0).normalize(),
                    Orientation::Left => Vec2::new(-1.0, -1.0).normalize(),
                    Orientation::DownLeft => Vec2::new(-1.0, 0.0).normalize(),
                    Orientation::Down => Vec2::new(-1.0, 1.0).normalize(),
                    Orientation::DownRight => Vec2::new(0.0, 1.0).normalize(),
                    Orientation::Right => Vec2::new(1.0, 1.0).normalize(),
                    Orientation::UpRight => Vec2::new(1.0, 0.0).normalize(),
                };
                player.shooting_state.shoot(shooting_direction);
            }
        } else {
            // Movement
            let current_speed = player.velocity.length();

            let natural_orientation = match key_code {
                KeyCode::Up => {
                    if player.velocity.y > 0.0 {
                        player.velocity.y -= DECELERATION;
                    } else {
                        player.velocity.y -= ACCELERATION;
                    }
                    Orientation::UpLeft
                }
                KeyCode::Down => {
                    if player.velocity.y < 0.0 {
                        player.velocity.y += DECELERATION;
                    } else {
                        player.velocity.y += ACCELERATION;
                    }
                    Orientation::DownRight
                }
                KeyCode::Left => {
                    if player.velocity.x > 0.0 {
                        player.velocity.x -= DECELERATION;
                    } else {
                        player.velocity.x -= ACCELERATION;
                    }
                    Orientation::DownLeft
                }
                KeyCode::Right => {
                    if player.velocity.x < 0.0 {
                        player.velocity.x += DECELERATION;
                    } else {
                        player.velocity.x += ACCELERATION;
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

pub struct Game {
    pub id: uuid::Uuid,
    pub red_data: GameData,
    pub blue_data: GameData,
    pub puck: Puck,
    pub skate_traces: Vec<U16Vec2>,
    pub timer: u128,
    pub last_tick: Instant,
    pub state: GameState,
    pub palette: Palette,
}

impl Game {
    pub const DURATION_MILLISECONDS: u128 = 90 * 1000;
    pub const STARTING_DELAY_MILLISECONDS: u64 = 3000;
    const AFTER_GOAL_DELAY_MILLISECONDS: u128 = 2000;
    pub fn new() -> Self {
        Self {
            red_data: GameData::new(GameSide::Red),
            blue_data: GameData::new(GameSide::Blue),
            puck: Puck::new(),
            skate_traces: vec![],
            id: uuid::Uuid::new_v4(),
            timer: 0,
            last_tick: Instant::now(),
            state: GameState::Starting {
                time: Instant::now(),
            },
            palette: Palette::default(),
        }
    }

    fn reset_after_goal(&mut self) {
        self.red_data.reset();
        self.blue_data.reset();
        self.puck = Puck::new();
        self.state = GameState::Starting {
            time: Instant::now(),
        };
        self.skate_traces.clear();
    }

    pub fn reset(&mut self) {
        self.reset_after_goal();
        self.red_data.score = 0;
        self.blue_data.score = 0;
        self.timer = 0;
    }

    fn update_running(&mut self, deltatime: f32) -> AppResult<()> {
        // If puck is colliding with player, the player gets the puck
        for player in [&self.red_data.player, &self.blue_data.player] {
            match are_colliding(&self.puck, player) {
                Some((ColliderType::Puck, ColliderType::Player)) => {
                    self.puck.possession = Some(player.side);
                }
                _ => {}
            }
        }

        for player in [&mut self.red_data.player, &mut self.blue_data.player] {
            player.update(deltatime);
            // Players bounce against walls
            player.maybe_bounce_against_rect(
                PITCH_INNER_RECT,
                COFFICIENT_OF_WALL_BOUNCING,
                RectSide::Inside,
            );

            // Players can't enter areas
            if are_colliding(player, &self.red_data.area).is_some() {
                println!("Collision {} and red area", player.side);
                inelastic_collision(player, &mut self.red_data.area, 0.01);
            } else if are_colliding(player, &self.blue_data.area).is_some() {
                println!("Collision {} and blue area", player.side);
                inelastic_collision(player, &mut self.blue_data.area, 0.01);
            }
        }

        // Players bounce off each other
        if let Some(colliders) = are_colliding(&self.red_data.player, &self.blue_data.player) {
            match colliders {
                (ColliderType::Catcher, ColliderType::Catcher) => {
                    // Catcher collision
                }
                _ => {
                    println!("Collision red and blue player");
                    inelastic_collision(&mut self.red_data.player, &mut self.blue_data.player, 0.95)
                }
            }
        }

        // Handle player collisions after rotation
        for player in [&mut self.red_data.player, &mut self.blue_data.player] {
            if let Some(new_orientation) = player.new_orientation {
                player.rotate(new_orientation);
                if are_colliding(player, &self.red_data.area).is_some() {
                    println!("Collision after rotation {} and red area", player.side);
                    player.undo_rotation();
                } else if are_colliding(player, &self.blue_data.area).is_some() {
                    println!("Collision after rotation {} and blue area", player.side);
                    player.undo_rotation();
                }
            }
        }
        if are_colliding(&self.red_data.player, &self.blue_data.player).is_some() {
            self.red_data.player.undo_rotation();
            self.blue_data.player.undo_rotation();
        }

        for player in [&self.red_data.player, &self.blue_data.player] {
            if player.position() != player.previous_position() {
                let head_position = player.position() + player.head_position_offset();
                self.skate_traces.push(head_position);
            }
        }

        while self.skate_traces.len() > SKATE_TRACE_LENGTH {
            self.skate_traces.remove(0);
        }

        // Goalies
        let red_goalie = &mut self.red_data.goalie;
        let blue_goalie = &mut self.blue_data.goalie;

        // Align goalies to player heads
        red_goalie.align_to_player(&self.red_data.player);
        blue_goalie.align_to_player(&self.blue_data.player);

        // Puck
        self.puck.update(deltatime);

        // Logic related to puck possession
        match are_colliding(&self.puck, &self.red_data.player) {
            Some((ColliderType::Puck, ColliderType::Catcher)) => {
                match self.puck.possession {
                    // Red got puck from blue
                    Some(GameSide::Blue) => {
                        if self.red_data.player.after_got_stolen_counter == 0.0 {
                            self.puck.possession = Some(GameSide::Red);
                            self.blue_data.player.after_got_stolen_counter =
                                AFTER_GOT_STOLEN_COUNTER_MILLISECONDS;
                        }
                    }
                    Some(GameSide::Red) => {
                        // Red player has the puck
                    }
                    None => {
                        self.puck.possession = Some(GameSide::Red);
                    }
                }
            }
            Some((ColliderType::Puck, ColliderType::Player)) => match self.puck.possession {
                None => {
                    inelastic_collision(&mut self.puck, &mut self.red_data.player, 0.75);
                }
                _ => {}
            },
            _ => {}
        }

        match are_colliding(&self.puck, &self.blue_data.player) {
            Some((ColliderType::Puck, ColliderType::Catcher)) => {
                match self.puck.possession {
                    // Blue got puck from red
                    Some(GameSide::Red) => {
                        if self.blue_data.player.after_got_stolen_counter == 0.0 {
                            self.puck.possession = Some(GameSide::Blue);
                            self.red_data.player.after_got_stolen_counter =
                                AFTER_GOT_STOLEN_COUNTER_MILLISECONDS;
                        }
                    }
                    Some(GameSide::Blue) => {
                        // Blue player has the puck
                    }
                    None => {
                        self.puck.possession = Some(GameSide::Blue);
                    }
                }
            }
            Some((ColliderType::Puck, ColliderType::Player)) => match self.puck.possession {
                None => {
                    inelastic_collision(&mut self.puck, &mut self.blue_data.player, 0.75);
                }
                _ => {}
            },
            _ => {}
        }

        // Puck positioning logic.
        // If the puck is in possession, it follows the player unless the player is shooting.
        if let Some(side) = self.puck.possession {
            let (player, other) = if side == GameSide::Red {
                (&mut self.red_data.player, &mut self.blue_data.player)
            } else {
                (&mut self.blue_data.player, &mut self.red_data.player)
            };

            if let Some(direction) = player.shooting_state.shot_towards(deltatime) {
                // If the player is shooting counter went to 0, the puck follows the shooting direction.
                player.after_shooting_counter = AFTER_SHOOTING_COUNTER_MILLISECONDS;
                player.new_orientation = Some(player.orientation.next());
                self.puck.possession = None;

                self.puck.velocity = direction * SHOOTING_POWER;
            } else {
                self.puck.attach_to_player(&player);
            }

            if other.shooting_state.is_shooting() {
                other.shooting_state.reset();
            }
        }

        // Puck bounces against goalie
        for goalie in [&mut self.red_data.goalie, &mut self.blue_data.goalie] {
            if are_colliding(&self.puck, goalie).is_some() {
                inelastic_collision(&mut self.puck, goalie, 0.8);
            }
        }

        // Check for goals!
        match self.puck.has_scored() {
            Some(GameSide::Red) => {
                self.red_data.score += 1;
                self.state = GameState::AfterGoal {
                    time: Instant::now(),
                    scored: GameSide::Red,
                };
                return Ok(());
            }
            Some(GameSide::Blue) => {
                self.blue_data.score += 1;
                self.state = GameState::AfterGoal {
                    time: Instant::now(),
                    scored: GameSide::Blue,
                };
                return Ok(());
            }
            None => {}
        }

        Ok(())
    }

    pub fn is_over(&self) -> bool {
        matches!(self.state, GameState::Ending { .. })
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, GameState::Running { .. })
    }

    pub fn handle_key_events(&mut self, side: GameSide, key_code: KeyCode) {
        match side {
            GameSide::Red => &mut self.red_data.handle_key_events(&mut self.puck, key_code),
            GameSide::Blue => &mut self.blue_data.handle_key_events(&mut self.puck, key_code),
        };
    }

    pub fn update(&mut self) -> AppResult<()> {
        let now = Instant::now();
        let deltatime = now.duration_since(self.last_tick).as_millis() as f32;

        match self.state {
            GameState::WaitingForPlayer => {}
            GameState::Starting { time } => {
                if time.elapsed() >= Duration::from_millis(Self::STARTING_DELAY_MILLISECONDS) {
                    self.state = GameState::Running;
                }
            }
            GameState::Running => {
                self.update_running(deltatime)?;
                self.timer += deltatime as u128;
                if self.timer > Self::DURATION_MILLISECONDS {
                    self.state = GameState::Ending {
                        time: Instant::now(),
                    };
                }
            }
            GameState::AfterGoal { time, scored: _ } => {
                if now.duration_since(time).as_millis() >= Self::AFTER_GOAL_DELAY_MILLISECONDS {
                    self.reset_after_goal();
                }
            }
            _ => {}
        }
        self.last_tick = now;

        Ok(())
    }

    pub fn image(&self) -> AppResult<RgbaImage> {
        let mut img = PITCH_IMAGES
            .get(&self.palette)
            .expect("Pitch image should exist")
            .clone();

        for trace in &self.skate_traces {
            img.put_pixel(
                trace.x as u32,
                trace.y as u32,
                self.palette.skate_trace_color(),
            );
        }

        let red_player = &self.red_data.player;
        let blue_player = &self.blue_data.player;
        let red_goalie = &self.red_data.goalie;
        let blue_goalie = &self.blue_data.goalie;

        img.copy_non_trasparent_from(
            red_player.image(self.palette),
            red_player.position().x as u32,
            red_player.position().y as u32,
        )?;

        img.copy_non_trasparent_from(
            red_goalie.image(self.palette),
            red_goalie.position().x as u32,
            red_goalie.position().y as u32,
        )?;

        img.copy_non_trasparent_from(
            blue_player.image(self.palette),
            blue_player.position().x as u32,
            blue_player.position().y as u32,
        )?;
        img.copy_non_trasparent_from(
            blue_goalie.image(self.palette),
            blue_goalie.position().x as u32,
            blue_goalie.position().y as u32,
        )?;

        img.copy_non_trasparent_from(
            &self.puck.image(self.palette),
            self.puck.position().x as u32,
            self.puck.position().y as u32,
        )?;

        Ok(img)
    }
}

#[cfg(test)]

mod test {
    use super::*;
    use crate::engine::goalie;
    use crate::traits::ColliderType;
    use core::time;
    use glam::{I16Vec2, U16Vec2};
    use image::Rgba;
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Root};
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::Config;
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Layout};
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    fn init() -> AppResult<()> {
        let logfile_path = store_path("sshattrick.log")?;
        let logfile = FileAppender::builder()
            .append(false)
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(logfile_path)?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;

        Ok(())
    }

    #[test]
    fn test_puck_position_with_rotation() -> AppResult<()> {
        let mut player = Player::new(GameSide::Red);
        player.set_position(U16Vec2::new(50, 40));
        let mut puck = Puck::new();

        puck.set_position(player.catcher_position());

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        let palette = Palette::Dark;

        for _ in 0..16 {
            puck.set_position(player.catcher_position());
            terminal.draw(|frame| {
                let mut img = PITCH_IMAGES
                    .get(&palette)
                    .expect("Pitch image should exist")
                    .clone();

                img.copy_non_trasparent_from(
                    &player.image(palette),
                    player.position().x as u32,
                    player.position().y as u32,
                )
                .unwrap();

                img.copy_non_trasparent_from(
                    &puck.image(palette),
                    puck.position().x as u32,
                    puck.position().y as u32,
                )
                .unwrap();

                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let info = Paragraph::new(format!("Orientation {}", player.orientation as u8));
                frame.render_widget(info, split[0]);

                let paragraph = Paragraph::new(img_to_lines(&img));
                frame.render_widget(paragraph, split[1]);
            })?;
            player.rotate(player.orientation.next());
            std::thread::sleep(time::Duration::from_millis(500));
        }

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_goalie_boundaries() -> AppResult<()> {
        let mut red_goalie = goalie::Goalie::new(GameSide::Red);
        let mut blue_goalie = goalie::Goalie::new(GameSide::Blue);

        println!("Goalie size: {}", blue_goalie.size());

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        let palette = Palette::Dark;
        std::thread::sleep(time::Duration::from_millis(500));

        for idx in 0..32 {
            terminal.draw(|frame| {
                let mut img = PITCH_IMAGES
                    .get(&palette)
                    .expect("Pitch image should exist")
                    .clone();

                img.copy_non_trasparent_from(
                    &red_goalie.image(palette),
                    red_goalie.position().x as u32,
                    red_goalie.position().y as u32,
                )
                .unwrap();

                img.copy_non_trasparent_from(
                    &blue_goalie.image(palette),
                    blue_goalie.position().x as u32,
                    blue_goalie.position().y as u32,
                )
                .unwrap();

                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let info = Paragraph::new(format!(
                    "Red position {} - Blue position {} - Size {} ",
                    red_goalie.position(),
                    blue_goalie.position(),
                    red_goalie.size(),
                ));
                frame.render_widget(info, split[0]);

                let paragraph = Paragraph::new(img_to_lines(&img));
                frame.render_widget(paragraph, split[1]);
            })?;

            let new_position = (red_goalie.position().as_i16vec2()
                + I16Vec2::new(0, if idx > 10 { 1 } else { -1 }))
            .as_u16vec2();
            red_goalie.set_position(new_position);

            let new_position = (blue_goalie.position().as_i16vec2()
                + I16Vec2::new(0, if idx > 10 { 1 } else { -1 }))
            .as_u16vec2();
            blue_goalie.set_position(new_position);
            std::thread::sleep(time::Duration::from_millis(250));
        }

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_goalie_position_with_rotation() -> AppResult<()> {
        let mut player = Player::new(GameSide::Red);
        player.set_position(U16Vec2::new(15, 40));
        let mut puck = Puck::new();

        puck.set_position(player.catcher_position());

        let mut goalie = goalie::Goalie::new(GameSide::Red);
        goalie.align_to_player(&player);

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        let palette = Palette::Dark;

        for idx in 0..16 {
            puck.set_position(player.catcher_position());
            goalie.align_to_player(&player);

            terminal
                .draw(|frame| {
                    let mut img = PITCH_IMAGES
                        .get(&palette)
                        .expect("Pitch image should exist")
                        .clone();

                    img.copy_non_trasparent_from(
                        &player.image(palette),
                        player.position().x as u32,
                        player.position().y as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &puck.image(palette),
                        puck.position().x as u32,
                        puck.position().y as u32,
                    )
                    .unwrap();

                    img.copy_non_trasparent_from(
                        &goalie.image(palette),
                        goalie.position().x as u32,
                        goalie.position().y as u32,
                    )
                    .unwrap();

                    let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                        .split(frame.area());

                    let info = Paragraph::new(format!("Orientation {}", player.orientation as u8));
                    frame.render_widget(info, split[0]);

                    let paragraph = Paragraph::new(img_to_lines(&img));
                    frame.render_widget(paragraph, split[1]);
                })
                .unwrap();

            let new_position =
                (player.position().as_i16vec2() + I16Vec2::new(0, idx % 3 - 1)).as_u16vec2();
            player.set_position(new_position);
            player.rotate(player.orientation.next());
            std::thread::sleep(time::Duration::from_millis(500));
        }

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_goalie_areas() -> AppResult<()> {
        let red_area = Area::new(GameSide::Red);
        let blue_area = Area::new(GameSide::Blue);
        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        let palette = Palette::Dark;
        terminal.draw(|frame| {
            let mut img = PITCH_IMAGES
                .get(&palette)
                .expect("Pitch image should exist")
                .clone();

            for area in [red_area, blue_area].iter() {
                for (point, collider_type) in area.hit_box().iter() {
                    let pixel = match collider_type {
                        ColliderType::GoalieAreaHorizontalSide => Rgba::from([255, 255, 0, 55]),
                        ColliderType::GoalieAreaVerticalSize => Rgba::from([0, 255, 255, 55]),
                        _ => unreachable!(),
                    };

                    let g_point = area.position() + point;
                    img.put_pixel(g_point.x as u32, g_point.y as u32, pixel);
                }
            }

            let split =
                Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(frame.area());

            let paragraph = Paragraph::new(img_to_lines(&img));
            frame.render_widget(paragraph, split[1]);
        })?;
        std::thread::sleep(time::Duration::from_millis(5000));

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_puck_boundaries() -> AppResult<()> {
        let mut game = Game::new();
        game.state = GameState::Running;

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        for _ in 0..100 {
            if let Err(e) = game.update() {
                println!("Update error: {e}");
            }
            terminal.draw(|frame| {
                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let image = match game.image() {
                    Ok(img) => img,
                    Err(e) => panic!("Update error: {e}"),
                };
                let paragraph = Paragraph::new(img_to_lines(&image));
                frame.render_widget(paragraph, split[1]);
            })?;
            std::thread::sleep(time::Duration::from_millis(50));
        }

        game.puck.set_velocity(Vec2::new(0.075, 0.0));

        for _ in 0..100 {
            if let Err(e) = game.update() {
                println!("Update error: {e}");
            }
            terminal.draw(|frame| {
                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let image = match game.image() {
                    Ok(img) => img,
                    Err(e) => panic!("Update error: {e}"),
                };
                let paragraph = Paragraph::new(img_to_lines(&image));
                frame.render_widget(paragraph, split[1]);
            })?;
            std::thread::sleep(time::Duration::from_millis(50));
        }

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_goal_areas() -> AppResult<()> {
        let mut puck = Puck::new();
        puck.set_position(U16Vec2::new(MAX_X - 20, 30));
        puck.set_velocity(Vec2::new(0.02, 0.0));
        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.clear()?;
        let palette = Palette::Dark;

        let mut last_tick = Instant::now();

        let mut score = 0;
        let mut y = 0.0;
        loop {
            let now = Instant::now();
            let deltatime = now.duration_since(last_tick).as_millis() as f32;

            puck.update(deltatime);

            terminal.draw(|frame| {
                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let mut img = PITCH_IMAGES
                    .get(&palette)
                    .expect("Pitch image should exist")
                    .clone();

                img.copy_non_trasparent_from(
                    &puck.image(palette),
                    puck.position().x as u32,
                    puck.position().y as u32,
                )
                .unwrap();

                let info = format!("Score {}", score);
                let paragraph = Paragraph::new(info);
                frame.render_widget(paragraph, split[0]);

                let paragraph = Paragraph::new(img_to_lines(&img));
                frame.render_widget(paragraph, split[1]);
            })?;

            if puck.has_scored().is_some() {
                score += 1;
                y += 1.0;
                puck.set_position(U16Vec2::new(MAX_X - 20, 30 + y as u16));
                puck.set_velocity(Vec2::new(0.05, 0.0));
            } else if puck.velocity.x < 0.0 {
                y += 1.0;
                puck.set_position(U16Vec2::new(MAX_X - 20, 30 + y as u16));
                puck.set_velocity(Vec2::new(0.05, 0.0));
            }

            if y > 30.0 {
                break;
            }

            std::thread::sleep(time::Duration::from_millis(20));
            last_tick = now;
        }

        // -2 because we want the inner height, +1 because of the puck size
        assert!(score == GOALIE_AREA_HEIGHT - 2 + 1);

        Ok(())
    }

    #[test]
    fn test_player_hitbox_with_rotation() -> AppResult<()> {
        let mut player = Player::new(GameSide::Red);
        player.set_position(U16Vec2::new(50, 40));
        let mut puck = Puck::new();

        puck.set_position(player.catcher_position());

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        let palette = Palette::Dark;

        for _ in 0..16 {
            puck.set_position(player.catcher_position());
            terminal.draw(|frame| {
                let mut img = PITCH_IMAGES
                    .get(&palette)
                    .expect("Pitch image should exist")
                    .clone();

                img.copy_non_trasparent_from(
                    &player.image(palette),
                    player.position().x as u32,
                    player.position().y as u32,
                )
                .unwrap();

                img.copy_non_trasparent_from(
                    &puck.image(palette),
                    puck.position().x as u32,
                    puck.position().y as u32,
                )
                .unwrap();

                for (point, collider_type) in player.hit_box().iter() {
                    let img_point = player.position() + point;
                    match collider_type {
                        ColliderType::Player => img.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([255, 55, 55, 255]),
                        ),
                        ColliderType::Stick => img.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([55, 255, 125, 255]),
                        ),
                        ColliderType::Catcher => img.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([55, 125, 255, 255]),
                        ),
                        _ => unreachable!(),
                    }
                }

                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let info = Paragraph::new(format!("Orientation {}", player.orientation as u8));
                frame.render_widget(info, split[0]);

                let paragraph = Paragraph::new(img_to_lines(&img));
                frame.render_widget(paragraph, split[1]);
            })?;
            player.rotate(player.orientation.next());
            std::thread::sleep(time::Duration::from_millis(500));
        }

        terminal.clear()?;

        Ok(())
    }

    #[test]
    fn test_player_puck_collisions() -> AppResult<()> {
        init()?;
        let mut game = Game::new();
        game.state = GameState::Running;

        // create crossterm terminal to stdout
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        terminal.clear()?;

        game.puck.set_position(U16Vec2::new(80, 40));
        game.puck.set_velocity(Vec2::new(0.1, 0.0));

        let start = Instant::now();

        loop {
            if let Err(e) = game.update() {
                println!("Update error: {e}");
            }

            terminal.draw(|frame| {
                let split = Layout::vertical([Constraint::Length(5), Constraint::Min(1)])
                    .split(frame.area());

                let player = &game.blue_data.player;

                if let Some(colliders) = are_colliding(player, &game.puck) {
                    let paragraph = Paragraph::new(format!("Collision detected: {:#?}", colliders));
                    frame.render_widget(paragraph, split[0]);
                }

                let mut image = match game.image() {
                    Ok(img) => img,
                    Err(e) => panic!("Update error: {e}"),
                };

                for (point, collider_type) in player.hit_box().iter() {
                    let img_point = player.position() + point;
                    match collider_type {
                        ColliderType::Player => image.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([255, 55, 55, 255]),
                        ),
                        ColliderType::Stick => image.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([55, 255, 125, 255]),
                        ),
                        ColliderType::Catcher => image.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([55, 125, 255, 255]),
                        ),
                        _ => unreachable!(),
                    }
                }

                for (point, collider_type) in game.puck.hit_box().iter() {
                    let img_point = game.puck.position() + point;
                    match collider_type {
                        ColliderType::Puck => image.put_pixel(
                            img_point.x as u32,
                            img_point.y as u32,
                            Rgba([0, 155, 255, 255]),
                        ),

                        _ => unreachable!(),
                    }
                }

                let paragraph = Paragraph::new(img_to_lines(&image));
                frame.render_widget(paragraph, split[1]);
            })?;
            // std::thread::sleep(time::Duration::from_millis(50));
            if start.elapsed() > Duration::from_millis(5000) {
                break;
            }
        }

        terminal.clear()?;

        Ok(())
    }
}
