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

fn load_image(path: &str) -> RgbaImage {
    read_image(path).unwrap_or_else(|_| panic!("Could not read {path}"))
}

fn load_single(path: &str, collider_type: ColliderType) -> ImageData {
    let image = load_image(path);
    let hit_box = get_hit_box_from_image(&image, collider_type, vec![]);
    ImageData {
        images: vec![image],
        hit_boxes: vec![hit_box],
    }
}

const PLAYER_COLLIDER_OVERRIDES: [(Rgba<u8>, ColliderType); 2] = [
    (Rgba([188, 188, 188, 255]), ColliderType::Stick),
    (Rgba([134, 134, 134, 255]), ColliderType::Catcher),
];

fn load_player_data(prefix: &str) -> ImageData {
    let mut images = Vec::with_capacity(Orientation::MAX);
    let mut hit_boxes = Vec::with_capacity(Orientation::MAX);
    for orientation in 1..=Orientation::MAX {
        let image = load_image(&format!("{prefix}{orientation}.png"));
        let hit_box = get_hit_box_from_image(
            &image,
            ColliderType::Player,
            PLAYER_COLLIDER_OVERRIDES.to_vec(),
        );
        images.push(image);
        hit_boxes.push(hit_box);
    }
    ImageData { images, hit_boxes }
}

pub static PLAYER_IMAGE_DATA: LazyLock<HashMap<GameSide, ImageData>> = LazyLock::new(|| {
    HashMap::from([
        (GameSide::Red, load_player_data("red")),
        (GameSide::Blue, load_player_data("blue")),
    ])
});

pub static GOALIE_IMAGE_DATA: LazyLock<HashMap<GameSide, ImageData>> = LazyLock::new(|| {
    HashMap::from([
        (
            GameSide::Red,
            load_single("red_goalie.png", ColliderType::Goalie),
        ),
        (
            GameSide::Blue,
            load_single("blue_goalie.png", ColliderType::Goalie),
        ),
    ])
});

pub static PUCKS_IMAGE_DATA: LazyLock<HashMap<Palette, ImageData>> = LazyLock::new(|| {
    HashMap::from([
        (Palette::Dark, load_single("puck_white.png", ColliderType::Puck)),
        (Palette::Light, load_single("puck_black.png", ColliderType::Puck)),
        (Palette::Basket, load_single("puck_white.png", ColliderType::Puck)),
        (Palette::Alt, load_single("puck_gold.png", ColliderType::Puck)),
    ])
});

pub static PITCH_IMAGES: LazyLock<HashMap<Palette, RgbaImage>> = LazyLock::new(|| {
    HashMap::from([
        (Palette::Dark, load_image("pitch_empty.png")),
        (Palette::Light, load_image("pitch_classic.png")),
        (Palette::Basket, load_image("pitch_basket.png")),
        (Palette::Alt, load_image("pitch_alt.png")),
    ])
});

pub static PITCH_LINES: LazyLock<HashMap<Palette, Vec<Line<'static>>>> = LazyLock::new(|| {
    PITCH_IMAGES
        .iter()
        .map(|(palette, image)| (*palette, img_to_lines(image)))
        .collect()
});

fn pixel_color(image: &RgbaImage, x: u32, y: u32) -> Option<Color> {
    if y >= image.height() {
        return None;
    }
    let p = image.get_pixel(x, y);
    if p[3] > 0 {
        Some(Color::Rgb(p[0], p[1], p[2]))
    } else {
        None
    }
}

fn span_from_halves(top: Option<Color>, btm: Option<Color>) -> Span<'static> {
    match (top, btm) {
        (None, None) => Span::raw(" "),
        (Some(c), None) => Span::styled("▀", Style::default().fg(c)),
        (None, Some(c)) => Span::styled("▄", Style::default().fg(c)),
        (Some(t), Some(b)) => Span::styled("▀", Style::default().fg(t).bg(b)),
    }
}

/// Re-render terminal cells in `[cell_x_range, cell_y_range)` directly from `image`
/// into `lines`, overwriting whatever was there.
pub fn rerender_cells(
    lines: &mut [Line<'static>],
    image: &RgbaImage,
    cell_x_range: std::ops::Range<u16>,
    cell_y_range: std::ops::Range<u16>,
) {
    let img_w = image.width();
    for cy_term in cell_y_range {
        let Some(line) = lines.get_mut(cy_term as usize) else {
            continue;
        };
        let py_top = (cy_term as u32) * 2;
        let py_btm = py_top + 1;
        for cx in cell_x_range.clone() {
            if (cx as u32) >= img_w {
                continue;
            }
            let top = pixel_color(image, cx as u32, py_top);
            let btm = pixel_color(image, cx as u32, py_btm);
            if let Some(s) = line.spans.get_mut(cx as usize) {
                *s = span_from_halves(top, btm);
            }
        }
    }
}
