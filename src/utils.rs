use crate::traits::{ColliderType, HitBox};
use crate::types::{GameSide, Orientation, Palette};
use crate::AppResult;
use anyhow::anyhow;
use glam::U16Vec2;
use image::error::{ParameterError, ParameterErrorKind};
use image::{ImageBuffer, ImageError, ImageReader, ImageResult, Pixel, Rgba, RgbaImage};
use include_dir::{include_dir, Dir};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{error::Error, io::Cursor};

pub static ASSETS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/");

pub fn store_path(filename: &str) -> AppResult<PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "frittura", "sshattrick")
        .ok_or(anyhow!("Failed to get directories"))?;
    let config_dirs = dirs.config_dir();
    if !config_dirs.exists() {
        std::fs::create_dir_all(config_dirs)?;
    }
    let path = config_dirs.join(filename);
    Ok(path)
}
pub trait ExtraImageUtils {
    fn copy_non_trasparent_from(
        &mut self,
        other: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        x: u32,
        y: u32,
    ) -> ImageResult<()>;
}

impl ExtraImageUtils for ImageBuffer<Rgba<u8>, Vec<u8>> {
    fn copy_non_trasparent_from(
        &mut self,
        other: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        x: u32,
        y: u32,
    ) -> ImageResult<()> {
        // Do bounds checking here so we can use the non-bounds-checking
        // functions to copy pixels.
        if self.width() < other.width() + x || self.height() < other.height() + y {
            return Err(ImageError::Parameter(ParameterError::from_kind(
                ParameterErrorKind::DimensionMismatch,
            )));
        }

        for k in 0..other.height() {
            for i in 0..other.width() {
                let p = other.get_pixel(i, k);
                if p[3] > 0 {
                    self.put_pixel(i + x, k + y, *p);
                }
            }
        }
        Ok(())
    }
}

fn read_image(path: &str) -> Result<RgbaImage, Box<dyn Error>> {
    let file = ASSETS_DIR.get_file(path);
    if file.is_none() {
        return Err(format!("File {} not found", path).into());
    }
    let img = ImageReader::new(Cursor::new(file.unwrap().contents()))
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    Ok(img)
}

fn get_hit_box_from_image(
    image: &RgbaImage,
    default_collider_type: ColliderType,
    override_collider_types: Vec<(Rgba<u8>, ColliderType)>,
) -> HitBox {
    let mut hit_box = HashMap::new();

    for x in 0..image.width() {
        for y in 0..image.height() {
            if let Some(pixel) = image.get_pixel_checked(x, y) {
                // If pixel is non-transparent.
                if pixel[3] > 0 {
                    let point = U16Vec2::new(x as u16, y as u16);
                    let mut overriden = false;
                    for &(rgba, collider_type) in override_collider_types.iter() {
                        if *pixel == rgba {
                            hit_box.insert(point, collider_type);
                            overriden = true;
                        }
                    }
                    if !overriden {
                        hit_box.insert(point, default_collider_type);
                    }
                }
            }
        }
    }

    hit_box.into()
}

pub fn img_to_lines<'a>(img: &RgbaImage) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = vec![];
    let width = img.width();
    let height = img.height();

    for y in (0..height - 1).step_by(2) {
        let mut line: Vec<Span> = vec![];

        for x in 0..width {
            let top_pixel = img.get_pixel(x, y).to_rgba();
            let btm_pixel = img.get_pixel(x, y + 1).to_rgba();
            if top_pixel[3] == 0 && btm_pixel[3] == 0 {
                line.push(Span::raw(" "));
                continue;
            }

            if top_pixel[3] > 0 && btm_pixel[3] == 0 {
                let [r, g, b, _] = top_pixel.0;
                let color = Color::Rgb(r, g, b);
                line.push(Span::styled("▀", Style::default().fg(color)));
            } else if top_pixel[3] == 0 && btm_pixel[3] > 0 {
                let [r, g, b, _] = btm_pixel.0;
                let color = Color::Rgb(r, g, b);
                line.push(Span::styled("▄", Style::default().fg(color)));
            } else {
                let [fr, fg, fb, _] = top_pixel.0;
                let fg_color = Color::Rgb(fr, fg, fb);
                let [br, bg, bb, _] = btm_pixel.0;
                let bg_color = Color::Rgb(br, bg, bb);
                line.push(Span::styled(
                    "▀",
                    Style::default().fg(fg_color).bg(bg_color),
                ));
            }
        }
        lines.push(Line::from(line));
    }
    // append last line if height is odd
    if height % 2 == 1 {
        let mut line: Vec<Span> = vec![];
        for x in 0..width {
            let top_pixel = img.get_pixel(x, height - 1).to_rgba();
            if top_pixel[3] == 0 {
                line.push(Span::raw(" "));
                continue;
            }
            let [r, g, b, _] = top_pixel.0;
            let color = Color::Rgb(r, g, b);
            line.push(Span::styled("▀", Style::default().fg(color)));
        }
        lines.push(Line::from(line));
    }

    lines
}

pub struct ImageData {
    pub images: Vec<RgbaImage>,
    pub hit_boxes: Vec<HitBox>,
}

pub static PLAYER_IMAGE_DATA: LazyLock<HashMap<GameSide, ImageData>> = LazyLock::new(|| {
    let mut data = HashMap::new();

    let mut red_images = vec![];
    let mut red_hit_boxes = vec![];
    let mut blue_images = vec![];
    let mut blue_hit_boxes = vec![];
    for orientation in 1..=Orientation::MAX {
        let image = read_image(format!("red{}.png", orientation).as_str())
            .expect(format!("Could not read red{}.png.", orientation).as_str());
        let hit_box = get_hit_box_from_image(
            &image,
            ColliderType::Player,
            vec![
                (Rgba::from([188, 188, 188, 255]), ColliderType::Stick),
                (Rgba::from([134, 134, 134, 255]), ColliderType::Catcher),
            ],
        );
        red_images.push(image);
        red_hit_boxes.push(hit_box);

        let image = read_image(format!("blue{}.png", orientation).as_str())
            .expect(format!("Could not read blue{}.png.", orientation).as_str());
        let hit_box = get_hit_box_from_image(
            &image,
            ColliderType::Player,
            vec![
                (Rgba::from([188, 188, 188, 255]), ColliderType::Stick),
                (Rgba::from([134, 134, 134, 255]), ColliderType::Catcher),
            ],
        );
        blue_images.push(image);
        blue_hit_boxes.push(hit_box);
    }
    data.insert(
        GameSide::Red,
        ImageData {
            images: red_images,
            hit_boxes: red_hit_boxes,
        },
    );
    data.insert(
        GameSide::Blue,
        ImageData {
            images: blue_images,
            hit_boxes: blue_hit_boxes,
        },
    );

    data
});

pub static GOALIE_IMAGE_DATA: LazyLock<HashMap<GameSide, ImageData>> = LazyLock::new(|| {
    let mut data = HashMap::new();
    let image = read_image("red_goalie.png").expect("Could not read red_goalie.png");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Goalie, vec![]);
    data.insert(
        GameSide::Red,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    let image = read_image("blue_goalie.png").expect("Could not read blue_goalie.png");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Goalie, vec![]);
    data.insert(
        GameSide::Blue,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    data
});

pub static PUCKS_IMAGE_DATA: LazyLock<HashMap<Palette, ImageData>> = LazyLock::new(|| {
    let mut data = HashMap::new();

    let image = read_image("puck_white.png").expect("Could not read puck_white.png.");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Puck, vec![]);
    data.insert(
        Palette::Dark,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    let image = read_image("puck_black.png").expect("Could not read puck_black.png.");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Puck, vec![]);
    data.insert(
        Palette::Light,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    let image = read_image("puck_white.png").expect("Could not read puck_white.png.");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Puck, vec![]);
    data.insert(
        Palette::Basket,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    let image = read_image("puck_gold.png").expect("Could not read puck_gold.png.");
    let hit_box = get_hit_box_from_image(&image, ColliderType::Puck, vec![]);
    data.insert(
        Palette::Alt,
        ImageData {
            images: vec![image],
            hit_boxes: vec![hit_box],
        },
    );

    data
});

pub static PITCH_IMAGES: LazyLock<HashMap<Palette, RgbaImage>> = LazyLock::new(|| {
    let mut data = HashMap::new();
    data.insert(
        Palette::Dark,
        read_image("pitch_empty.png").expect("Could not read pitch_empty.png."),
    );
    data.insert(
        Palette::Light,
        read_image("pitch_classic.png").expect("Could not read pitch_classic.png."),
    );
    data.insert(
        Palette::Basket,
        read_image("pitch_basket.png").expect("Could not read pitch_basket.png."),
    );
    data.insert(
        Palette::Alt,
        read_image("pitch_alt.png").expect("Could not read pitch_alt.png."),
    );

    data
});
