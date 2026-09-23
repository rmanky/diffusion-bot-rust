use std::collections::HashMap;
use std::io::Cursor;

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, RgbImage};

const BACKGROUND: Rgb<u8> = Rgb([11, 15, 21]);
const TEXT: Rgb<u8> = Rgb([237, 242, 247]);
const MUTED: Rgb<u8> = Rgb([170, 182, 194]);
const BLANK: Rgb<u8> = Rgb([32, 42, 52]);
const CELL: u32 = 9;
const CELL_GAP: u32 = 1;
const ROWS_PER_COLUMN: usize = 7;
const HORIZONTAL_MARGIN: u32 = 24;
const BOTTOM_MARGIN: u32 = 24;

pub(super) struct ScoreDay {
    pub(super) timestamp: String,
    pub(super) scores: HashMap<String, u32>,
}

struct Player {
    id: &'static str,
    name: &'static str,
    color: Rgb<u8>,
    average: f32,
}

pub(super) fn render_png(
    days: &[ScoreDay],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if days.is_empty() {
        return Err("cannot render an empty score history".into());
    }

    let mut players = vec![
        player("150725833957441536", "@rmanky", [98, 223, 186], days),
        player("302973340371517441", "@Raúl", [255, 196, 107], days),
        player("481280459058184204", "@troyotter", [131, 173, 255], days),
        player("656347629524877312", "@aaron_", [244, 140, 170], days),
    ];
    players.sort_by(|a, b| a.average.total_cmp(&b.average));

    let columns = days.len().div_ceil(ROWS_PER_COLUMN);
    let grid_width = columns as u32 * (CELL + CELL_GAP) - CELL_GAP;
    let width = HORIZONTAL_MARGIN * 2 + grid_width;

    let eyebrow_height = text_height(2);
    let title_height = text_height(4);
    let metadata_height = text_height(2);
    let legend_height = text_height(3);
    let player_label_height = text_height(3);
    let grid_height = ROWS_PER_COLUMN as u32 * CELL + (ROWS_PER_COLUMN as u32 - 1) * CELL_GAP;
    let player_gap = 22;
    let timeline_gap = 24;
    let player_block_height = player_label_height + 10 + grid_height;

    let mut y = 24 + eyebrow_height + 8 + title_height + 8 + metadata_height + 16;
    y += legend_height + 19;
    y += players.len() as u32 * player_block_height;
    y += (players.len() as u32 - 1) * player_gap;
    y += timeline_gap + legend_height + BOTTOM_MARGIN;
    let height = y;

    let mut image = ImageBuffer::from_pixel(width, height, BACKGROUND);
    let mut cursor_y = 24;
    draw_text(
        &mut image,
        HORIZONTAL_MARGIN,
        cursor_y,
        "WORDLE / HISTORY",
        2,
        Rgb([98, 223, 186]),
    );
    cursor_y += eyebrow_height + 8;
    draw_text(
        &mut image,
        HORIZONTAL_MARGIN,
        cursor_y,
        "SCORE GRID",
        4,
        TEXT,
    );
    cursor_y += title_height + 8;
    draw_text(
        &mut image,
        HORIZONTAL_MARGIN,
        cursor_y,
        &format!("{} POSTS", days.len()),
        2,
        MUTED,
    );
    cursor_y += metadata_height + 16;

    for score in 1..=7 {
        let x = HORIZONTAL_MARGIN + (score - 1) * 38;
        fill_rect(&mut image, x, cursor_y, 14, 14, score_color(score));
        let label = if score == 7 {
            "X".to_string()
        } else {
            score.to_string()
        };
        draw_text(&mut image, x + 21, cursor_y, &label, 3, TEXT);
    }
    cursor_y += legend_height + 19;

    for (rank, player) in players.iter().enumerate() {
        draw_text(
            &mut image,
            HORIZONTAL_MARGIN,
            cursor_y,
            &format!("{:02} {}", rank + 1, player.name),
            3,
            player.color,
        );
        draw_text_right(
            &mut image,
            HORIZONTAL_MARGIN + grid_width,
            cursor_y,
            &format!("{:.2}", player.average),
            3,
            TEXT,
        );

        let grid_y = cursor_y + player_label_height + 10;
        for (post_index, day) in days.iter().enumerate() {
            let x = HORIZONTAL_MARGIN + (post_index / ROWS_PER_COLUMN) as u32 * (CELL + CELL_GAP);
            let row = post_index % ROWS_PER_COLUMN;
            let y = grid_y + row as u32 * (CELL + CELL_GAP);
            let fill = day
                .scores
                .get(player.id)
                .map(|score| score_color(*score))
                .unwrap_or(BLANK);
            fill_rect(&mut image, x, y, CELL, CELL, fill);
        }

        cursor_y += player_block_height + player_gap;
    }

    let timeline_y = height - BOTTOM_MARGIN - legend_height;
    draw_text(
        &mut image,
        HORIZONTAL_MARGIN,
        timeline_y,
        &month_label(&days[0].timestamp),
        3,
        MUTED,
    );
    draw_text_center(
        &mut image,
        HORIZONTAL_MARGIN + grid_width / 2,
        timeline_y,
        &month_label(&days[days.len() / 2].timestamp),
        3,
        MUTED,
    );
    draw_text_right(
        &mut image,
        HORIZONTAL_MARGIN + grid_width,
        timeline_y,
        &month_label(&days[days.len() - 1].timestamp),
        3,
        MUTED,
    );

    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut output, ImageFormat::Png)?;
    Ok(output.into_inner())
}

fn player(id: &'static str, name: &'static str, color: [u8; 3], days: &[ScoreDay]) -> Player {
    let mut total = 0u32;
    let mut played = 0usize;
    for day in days {
        if let Some(score) = day.scores.get(id) {
            total += *score;
            played += 1;
        }
    }
    Player {
        id,
        name,
        color: Rgb(color),
        average: if played == 0 {
            0.0
        } else {
            total as f32 / played as f32
        },
    }
}

fn score_color(score: u32) -> Rgb<u8> {
    match score {
        1 => Rgb([55, 189, 141]),
        2 => Rgb([91, 197, 130]),
        3 => Rgb([130, 190, 115]),
        4 => Rgb([184, 174, 97]),
        5 => Rgb([217, 155, 84]),
        6 => Rgb([236, 128, 95]),
        _ => Rgb([190, 96, 121]),
    }
}

fn month_label(timestamp: &str) -> String {
    let month = timestamp
        .get(5..7)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let year = timestamp.get(2..4).unwrap_or("??");
    let name = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ]
    .get(month.saturating_sub(1))
    .unwrap_or(&"???");
    format!("{} '{}'", name, year)
}

fn fill_rect(image: &mut RgbImage, x: u32, y: u32, width: u32, height: u32, color: Rgb<u8>) {
    for py in y..(y + height).min(image.height()) {
        for px in x..(x + width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_text(image: &mut RgbImage, x: u32, y: u32, text: &str, scale: u32, color: Rgb<u8>) {
    let mut cursor_x = x;
    for character in text.chars() {
        let normalized = match character {
            'ú' | 'Ú' => 'U',
            other => other.to_ascii_uppercase(),
        };
        let bitmap = glyph(normalized);
        for (row, bits) in bitmap.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    fill_rect(
                        image,
                        cursor_x + column * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        if character == 'ú' || character == 'Ú' {
            fill_rect(image, cursor_x + 3 * scale, y, scale, scale, color);
            fill_rect(
                image,
                cursor_x + 4 * scale,
                y.saturating_sub(scale),
                scale,
                scale,
                color,
            );
        }
        cursor_x += 6 * scale;
    }
}

fn draw_text_right(
    image: &mut RgbImage,
    right: u32,
    y: u32,
    text: &str,
    scale: u32,
    color: Rgb<u8>,
) {
    let width = text.chars().count() as u32 * 6 * scale - scale;
    draw_text(image, right.saturating_sub(width), y, text, scale, color);
}

fn draw_text_center(
    image: &mut RgbImage,
    center: u32,
    y: u32,
    text: &str,
    scale: u32,
    color: Rgb<u8>,
) {
    let width = text.chars().count() as u32 * 6 * scale - scale;
    draw_text(
        image,
        center.saturating_sub(width / 2),
        y,
        text,
        scale,
        color,
    );
}

fn text_height(scale: u32) -> u32 {
    7 * scale
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [31, 4, 4, 4, 4, 4, 31],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '@' => [14, 17, 23, 21, 23, 16, 14],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '\'' => [12, 12, 4, 0, 0, 0, 0],
        _ => [0; 7],
    }
}
