use image::{Pixel, RgbaImage};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use sshattrick_core::types::Palette;
use sshattrick_core::utils::PITCH_IMAGES;
use std::collections::HashMap;
use std::sync::LazyLock;

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
