use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::io::Cursor;

use anyhow::{anyhow, ensure, Context, Result};
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::mono_font::{iso_8859_1, MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Drawable, Pixel, RgbColor};
use embedded_graphics::text::{Baseline, Text};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, RgbImage};
use jiff::{civil::Date, Timestamp, ToSpan};

const BACKGROUND: Rgb<u8> = Rgb([11, 15, 21]);
const TEXT: Rgb<u8> = Rgb([237, 242, 247]);
const MUTED: Rgb<u8> = Rgb([170, 182, 194]);
const BLANK: Rgb<u8> = Rgb([32, 42, 52]);
const CELL: u32 = 9;
const CELL_GAP: u32 = 1;
const HORIZONTAL_MARGIN: u32 = 24;
const WEEKDAY_LABEL_WIDTH: u32 = 5;
const WEEKDAY_LABEL_GAP: u32 = 4;
const BOTTOM_MARGIN: u32 = 24;
const LEGEND_SWATCH_SIZE: u32 = 14;
const LEGEND_LABEL_GAP: u32 = 5;
const LEGEND_ITEM_GAP: u32 = 12;
const WEEKDAY_LABELS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];
const FONT_SMALL: &MonoFont = &iso_8859_1::FONT_7X13;
const FONT_MEDIUM: &MonoFont = &iso_8859_1::FONT_9X18;
const FONT_LARGE: &MonoFont = &iso_8859_1::FONT_10X20;
const FONT_WEEKDAY: &MonoFont = &iso_8859_1::FONT_5X7;

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

struct CalendarDay {
    date: Date,
    scores: Option<HashMap<String, u32>>,
}

pub(super) fn render_png(days: &[ScoreDay]) -> Result<Vec<u8>> {
    ensure!(!days.is_empty(), "cannot render an empty score history");

    let calendar_days = calendar_days(days)?;
    let wordle_dates: Vec<Date> = calendar_days
        .iter()
        .filter(|day| day.scores.is_some())
        .map(|day| day.date)
        .collect();
    let mut players = vec![
        player(
            "150725833957441536",
            "@rmanky",
            [98, 223, 186],
            &calendar_days,
        ),
        player(
            "302973340371517441",
            "@Raúl",
            [255, 196, 107],
            &calendar_days,
        ),
        player(
            "481280459058184204",
            "@troyotter",
            [131, 173, 255],
            &calendar_days,
        ),
        player(
            "656347629524877312",
            "@aaron_",
            [244, 140, 170],
            &calendar_days,
        ),
    ];
    players.sort_by(|a, b| a.average.total_cmp(&b.average));

    let columns = calendar_days.len() / 7;
    let grid_width = columns as u32 * (CELL + CELL_GAP) - CELL_GAP;
    let grid_x = HORIZONTAL_MARGIN + WEEKDAY_LABEL_WIDTH + WEEKDAY_LABEL_GAP;
    let grid_right = grid_x + grid_width;
    let width = grid_right + HORIZONTAL_MARGIN;

    let eyebrow_height = text_height(FONT_SMALL);
    let title_height = text_height(FONT_LARGE);
    let metadata_height = text_height(FONT_SMALL);
    let legend_height = text_height(FONT_MEDIUM).max(LEGEND_SWATCH_SIZE);
    let player_label_height = text_height(FONT_MEDIUM);
    let grid_height = 7 * CELL + 6 * CELL_GAP;
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
        grid_x,
        cursor_y,
        "WORDLE / HISTORY",
        FONT_SMALL,
        Rgb([98, 223, 186]),
    );
    cursor_y += eyebrow_height + 8;
    draw_text(&mut image, grid_x, cursor_y, "SCORE GRID", FONT_LARGE, TEXT);
    cursor_y += title_height + 8;
    draw_text(
        &mut image,
        grid_x,
        cursor_y,
        &format!(
            "{} DAYS",
            calendar_days
                .iter()
                .filter(|day| day.scores.is_some())
                .count()
        ),
        FONT_SMALL,
        MUTED,
    );
    cursor_y += metadata_height + 16;

    for score in 1..=7 {
        let label = if score == 7 {
            "X".to_owned()
        } else {
            score.to_string()
        };
        let item_width = LEGEND_SWATCH_SIZE
            + LEGEND_LABEL_GAP
            + text_width(&label, FONT_MEDIUM)
            + LEGEND_ITEM_GAP;
        let x = grid_x + (score - 1) * item_width;
        let swatch_y = cursor_y + (legend_height - LEGEND_SWATCH_SIZE) / 2;
        fill_rect(
            &mut image,
            x,
            swatch_y,
            LEGEND_SWATCH_SIZE,
            LEGEND_SWATCH_SIZE,
            score_color(score),
        );
        let label_y = cursor_y + (legend_height - text_height(FONT_MEDIUM)) / 2;
        draw_text(
            &mut image,
            x + LEGEND_SWATCH_SIZE + LEGEND_LABEL_GAP,
            label_y,
            &label,
            FONT_MEDIUM,
            TEXT,
        );
    }
    cursor_y += legend_height + 19;

    for (rank, player) in players.iter().enumerate() {
        draw_text(
            &mut image,
            grid_x,
            cursor_y,
            &format!("{:02} {}", rank + 1, player.name),
            FONT_MEDIUM,
            player.color,
        );
        draw_text_right(
            &mut image,
            grid_right,
            cursor_y,
            &format!("{:.2}", player.average),
            FONT_MEDIUM,
            TEXT,
        );

        let grid_y = cursor_y + player_label_height + 10;
        for row in 0..7 {
            let label_y =
                grid_y + row as u32 * (CELL + CELL_GAP) + (CELL - text_height(FONT_WEEKDAY)) / 2;
            draw_text(
                &mut image,
                HORIZONTAL_MARGIN,
                label_y,
                WEEKDAY_LABELS[row],
                FONT_WEEKDAY,
                MUTED,
            );
        }

        for (day_index, day) in calendar_days.iter().enumerate() {
            let column = day_index / 7;
            let row = day_index % 7;
            let x = grid_x + column as u32 * (CELL + CELL_GAP);
            let y = grid_y + row as u32 * (CELL + CELL_GAP);
            if let Some(scores) = &day.scores {
                let fill = scores
                    .get(player.id)
                    .map(|score| score_color(*score))
                    .unwrap_or(BLANK);
                fill_rect(&mut image, x, y, CELL, CELL, fill);
            }
        }

        cursor_y += player_block_height + player_gap;
    }

    let timeline_y = height - BOTTOM_MARGIN - legend_height;
    draw_text(
        &mut image,
        grid_x,
        timeline_y,
        &month_label(wordle_dates[0]),
        FONT_SMALL,
        MUTED,
    );
    draw_text_center(
        &mut image,
        grid_x + grid_width / 2,
        timeline_y,
        &month_label(wordle_dates[wordle_dates.len() / 2]),
        FONT_SMALL,
        MUTED,
    );
    draw_text_right(
        &mut image,
        grid_right,
        timeline_y,
        &month_label(wordle_dates[wordle_dates.len() - 1]),
        FONT_SMALL,
        MUTED,
    );

    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut output, ImageFormat::Png)
        .context("encoding score grid as PNG")?;
    Ok(output.into_inner())
}

fn player(id: &'static str, name: &'static str, color: [u8; 3], days: &[CalendarDay]) -> Player {
    let mut total = 0u32;
    let mut played = 0usize;
    for day in days {
        if let Some(score) = day.scores.as_ref().and_then(|scores| scores.get(id)) {
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

fn calendar_days(days: &[ScoreDay]) -> Result<Vec<CalendarDay>> {
    let mut scores_by_date: BTreeMap<Date, HashMap<String, u32>> = BTreeMap::new();
    for day in days {
        let date = eastern_date(&day.timestamp)?;
        let scores = scores_by_date.entry(date).or_default();
        // If Discord has multiple reports for a Wordle date, retain the latest
        // score for each player while keeping one cell for that puzzle day.
        scores.extend(
            day.scores
                .iter()
                .map(|(user_id, score)| (user_id.clone(), *score)),
        );
    }

    let first_date = *scores_by_date
        .first_key_value()
        .ok_or_else(|| anyhow!("cannot render an empty score history"))?
        .0;
    let last_date = *scores_by_date
        .last_key_value()
        .ok_or_else(|| anyhow!("cannot render an empty score history"))?
        .0;
    let first_sunday =
        first_date.checked_sub(i64::from(first_date.weekday().to_sunday_zero_offset()).days())?;
    let last_saturday = last_date
        .checked_add((6 - i64::from(last_date.weekday().to_sunday_zero_offset())).days())?;

    let mut calendar_days = Vec::new();
    let mut date = first_sunday;
    while date <= last_saturday {
        calendar_days.push(CalendarDay {
            date,
            scores: scores_by_date.remove(&date),
        });
        date = date.checked_add(1.day())?;
    }
    Ok(calendar_days)
}

fn eastern_date(timestamp: &str) -> Result<Date> {
    let timestamp: Timestamp = timestamp.parse().context("parsing Wordle timestamp")?;
    Ok(timestamp
        .in_tz("America/New_York")
        .context("converting timestamp to Eastern time")?
        .date())
}

fn month_label(date: Date) -> String {
    let name = match date.month() {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        12 => "DEC",
        _ => "???",
    };
    format!("{} '{:02}", name, date.year().rem_euclid(100))
}

fn fill_rect(image: &mut RgbImage, x: u32, y: u32, width: u32, height: u32, color: Rgb<u8>) {
    for py in y..(y + height).min(image.height()) {
        for px in x..(x + width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_text(image: &mut RgbImage, x: u32, y: u32, text: &str, font: &MonoFont, color: Rgb<u8>) {
    let text = text.to_uppercase();
    let color = Rgb888::new(color[0], color[1], color[2]);
    let style = MonoTextStyle::new(font, color);
    let mut target = ImageDrawTarget(image);
    Text::with_baseline(&text, Point::new(x as i32, y as i32), style, Baseline::Top)
        .draw(&mut target)
        .unwrap();
}

fn draw_text_right(
    image: &mut RgbImage,
    right: u32,
    y: u32,
    text: &str,
    font: &MonoFont,
    color: Rgb<u8>,
) {
    let width = text_width(text, font);
    draw_text(image, right.saturating_sub(width), y, text, font, color);
}

fn draw_text_center(
    image: &mut RgbImage,
    center: u32,
    y: u32,
    text: &str,
    font: &MonoFont,
    color: Rgb<u8>,
) {
    let width = text_width(text, font);
    draw_text(
        image,
        center.saturating_sub(width / 2),
        y,
        text,
        font,
        color,
    );
}

fn text_width(text: &str, font: &MonoFont) -> u32 {
    text.chars().count() as u32 * (font.character_size.width + font.character_spacing)
}

fn text_height(font: &MonoFont) -> u32 {
    font.character_size.height
}

struct ImageDrawTarget<'a>(&'a mut RgbImage);

impl OriginDimensions for ImageDrawTarget<'_> {
    fn size(&self) -> Size {
        Size::new(self.0.width(), self.0.height())
    }
}

impl DrawTarget for ImageDrawTarget<'_> {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.y >= 0 {
                let (x, y) = (point.x as u32, point.y as u32);
                if x < self.0.width() && y < self.0.height() {
                    self.0
                        .put_pixel(x, y, Rgb([color.r(), color.g(), color.b()]));
                }
            }
        }
        Ok(())
    }
}
