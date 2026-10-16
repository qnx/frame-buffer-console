// Copyright (c) 2026 BlackBerry Limited.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod framebuffer;
mod screen;
mod font;
pub mod cursor;
mod escape;

use framebuffer::FrameBuffer;
use screen::Screen;
use font::Font;
use cursor::{Cursor, CursorDir};

use std::sync::{Arc, Mutex, Condvar};
use std::thread;

/// Top-level graphical UI object.
pub struct Display {
    obj: Arc<DisplayObj>
}

/// Internal display object.
struct DisplayObj {
    /// Display width, in pixels.
    width: u32,
    /// Display height, in pixels.
    height: u32,
    /// Console text contents.
    screen: Mutex<Screen>,
    /// Display update signalling object.
    cond: Condvar,
    /// Typeface used for drawing text.
    font: &'static Font
}

/// A character on the screen.
/// Every position on the screen is represented by two bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone, Copy, Debug)]
pub struct ScreenChar {
    pub chr: u8,
    pub attr: u8
}

const PALETTE: [u32; 16] = [
    0xFF000000, // 0 - black
    0xFFAA0000, // 1 - red
    0xFF00AA00, // 2 - green
    0xFFAA5500, // 3 - brown
    0xFF0000AA, // 4 - blue
    0xFFAA00AA, // 5 - magenta
    0xFF00AAAA, // 6 - cyan
    0xFFAAAAAA, // 7 - white
    0xFF555555, // 8 - grey
    0xFFFF5555, // 9 - light red
    0xFF55FF55, // 10 - light green
    0xFFFFFF55, // 11 - yellow
    0xFF5555FF, // 12 - light blue
    0xFFFF55FF, // 13 - light magenta
    0xFF55FFFF, // 14 - light cyan
    0xFFFFFFFF, // 15 - bright white
];

pub const DEFAULT_ATTR: u8 = 0x7;

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32
}

impl Display {
    pub fn new(double_buffer: bool, screen_size: Option<(u32, u32)>) -> Result<Display, ()> {
        // Map the frame buffer.
        let framebuffer = FrameBuffer::new(double_buffer)?;

        // Choose a font based on the frame buffer's geometry.
        let font = if (framebuffer.height * 2) < framebuffer.width {
            &font::FONT_9X8
        } else {
            &font::FONT_9X16
        };

        // Calculate the number of columns and rows.
        let mut columns = framebuffer.width / font.width as u32;
        let mut rows = framebuffer.height / font.height as u32;

        // Allow the user to override the screen size.
        if let Some(screen_size) = screen_size {
            columns = std::cmp::min(columns, screen_size.0);
            rows = std::cmp::min(rows, screen_size.1);
        }

        // Construct the screen object.
        let cursor = Cursor { col: 0, row: 0, top: font.height - 1, bottom: font.height - 1 };
        let screen = Screen::new(columns, rows, cursor)?;
        let screen = Mutex::<Screen>::new(screen);

        let display = Display {
            obj: Arc::<DisplayObj>::new(DisplayObj {
                width: framebuffer.width,
                height: framebuffer.height,
                screen,
                cond: Condvar::new(),
                font
            })
        };

        // Start the screen update thread.
        let cloned_display = display.obj.clone();
        thread::spawn(move || { screen_update_thread(cloned_display, framebuffer); });
        Ok(display)
    }

    pub fn clone(&self) -> Display {
        Display { obj: self.obj.clone() }
    }

    pub fn insert_at_cursor(&mut self, chrs: &[u8]) {
        let mut screen = self.obj.screen.lock().unwrap();
        screen.insert_at_cursor(chrs);
        self.obj.cond.notify_one();
    }

    pub fn insert_line(&mut self, chrs: &[u8], row: u32, col: u32, cursor_col: u32) {
        let mut screen = self.obj.screen.lock().unwrap();
        screen.insert_line(chrs, row, col, cursor_col);
        self.obj.cond.notify_one();
    }

    pub fn move_in_line(&mut self, dir: CursorDir) {
        let mut screen = self.obj.screen.lock().unwrap();
        screen.move_in_line(dir);
        self.obj.cond.notify_one();
    }

    pub fn screen_size(&self) -> (u32, u32) {
        let screen = self.obj.screen.lock().unwrap();
        (screen.columns, screen.rows)
    }

    /// Get the cursor column and row.
    pub fn get_cursor_pos(&self) -> (u32, u32) {
        let screen = self.obj.screen.lock().unwrap();
        (screen.cursor.col, screen.cursor.row)
    }

    pub fn framebuffer_size(&self) -> (u32, u32) {
        (self.obj.width, self.obj.height)
    }
}

/// Thread for drawing the contents of the screen whenever these are updated.
fn screen_update_thread(display: Arc<DisplayObj>, framebuffer: FrameBuffer) {
    loop {
        // Throttle drawing to ~30FPS (must not be holding the screen's lock).
        // This provides smoother drawing with successive screen updates.
        thread::sleep(std::time::Duration::from_millis(33));

        let mut screen = display.screen.lock().unwrap();
        loop {
            // Check if an update is required.
            if let Some(rect) = screen.get_rect() {
                // Yes, redraw.
                draw_screen(&display, &screen, &framebuffer, &rect);
                break;
            }

            // No, wait for a signal.
            screen = display.cond.wait(screen).unwrap();
        }
    }
}

/// Draw a portion of the screen as defined by the given rectangle.
fn draw_screen(display: &DisplayObj, screen: &Screen, framebuffer: &FrameBuffer, rect: &Rect) {
    for row in rect.top..=rect.bottom {
        for col in rect.left..=rect.right {
            draw_glyph(&display, screen, framebuffer, col, row);
        }
    }

    framebuffer.update();
}

/// Draw the glyph of a given character into the frame buffer at the specified column and row.
fn draw_glyph(display: &DisplayObj, screen: &Screen, framebuffer: &FrameBuffer, col: u32,
              row: u32) {
    let font = display.font;
    let chr = screen.char_at(col, row);

    // Don't render past right side of the frame.
    let font_width = font.width as u64;
    let start_x = col as u64 * font_width;
    let start_y = row as u64 * font.height as u64;
    if (start_x + font_width) > framebuffer.width as u64 {
        return;
    }

    let is_cursor = (col == screen.cursor.col) && (row == screen.cursor.row);

    // Determine foreground and background colours.
    let mut fg = PALETTE[(chr.attr & 0xf) as usize];
    let mut bg = PALETTE[((chr.attr >> 4) & 0xf) as usize];

    // Find the starting pixel of the glyph in the frame buffer.
    let stride = framebuffer.stride as u64;
    let mut pixel = start_y * stride;

    // Draw the glyph itself.
    let mut bitidx = chr.chr as u32 * font.wbits as u32 * font.hbits as u32;
    for i in 0..font.hbits {
        let mut xoff = start_x;

        if is_cursor {
            // Drawing glyph in the cursor position.
            if (i >= screen.cursor.top) && (i <= screen.cursor.bottom) {
                // Inside block, use inverted colours.
                fg = PALETTE[((chr.attr >> 4) & 0xf) as usize];
                bg = PALETTE[(chr.attr & 0xf) as usize];
            } else {
                // Outside block, use normal colours.
                fg = PALETTE[(chr.attr & 0xf) as usize];
                bg = PALETTE[((chr.attr >> 4) & 0xf) as usize];
            }
        }

        // Draw glyph line.
        for _ in 0..font.wbits {
            let pixel_colour = match is_foreground(font, bitidx) {
                true => fg,
                false => bg
            };

            framebuffer.set_pixel(pixel + xoff, pixel_colour);
            xoff += 1;
            bitidx += 1;
        }

        // Draw space after glyph.
        for _j in font.wbits..font.width {
            let pixel_colour = if (chr.chr >= 0xb0) && (chr.chr <= 0xdf) {
                // Support for Line Graphics Enable.
                // Replicate previous pixel.
                match is_foreground(font, bitidx - 1) {
                    true => fg,
                    false => bg
                }
            } else {
                bg
            };

            framebuffer.set_pixel(pixel + xoff, pixel_colour);
            xoff += 1;
        }

        pixel += stride as u64;
    }
}

fn is_foreground(font: &Font, bitidx: u32) -> bool {
    (font.bitmap[(bitidx / 8) as usize] & (1 << (7 - (bitidx % 8)))) != 0
}
