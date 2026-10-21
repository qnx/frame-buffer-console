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
    font: &'static Font,
    /// Typeface used for drawing blocks.
    block_font: &'static Font
}

/// A character on the screen.
/// Every position on the screen is represented by four bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone, Copy, Debug)]
pub struct ScreenChar {
    pub chr: u8,
    pub attr: ScreenCharAttr
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenCharAttr {
    pub fg: u8,
    pub bg: u8,
    pub extra: u8
}

/// 256 colour palette.
/// Following ANSI, the first 16 colours are the basic palette, 16-232 come from a 6x6x6 colour
/// cube, and 232-255 are greyscale.
const PALETTE: [u32; 256] = [
    0xff000000, // 0 - black
    0xffaa0000, // 1 - red
    0xff00aa00, // 2 - green
    0xffaa5500, // 3 - brown
    0xff0000aa, // 4 - blue
    0xffaa00aa, // 5 - magenta
    0xff00aaaa, // 6 - cyan
    0xffaaaaaa, // 7 - white
    0xff555555, // 8 - grey
    0xffff5555, // 9 - light red
    0xff55ff55, // 10 - light green
    0xffffff55, // 11 - yellow
    0xff5555ff, // 12 - light blue
    0xffff55ff, // 13 - light magenta
    0xff55ffff, // 14 - light cyan
    0xffffffff, // 15 - bright white
    0xff000000,
    0xff01072a,
    0xff020e55,
    0xff031580,
    0xff041caa,
    0xff0523d5,
    0xff072a00,
    0xff08312a,
    0xff093855,
    0xff0a4080,
    0xff0b47aa,
    0xff0d4ed5,
    0xff0e5500,
    0xff0f5c2a,
    0xff106355,
    0xff116a80,
    0xff1271aa,
    0xff1478d5,
    0xff158000,
    0xff16872a,
    0xff178e55,
    0xff189580,
    0xff1a9caa,
    0xff1ba3d5,
    0xff1caa00,
    0xff1db12a,
    0xff1eb855,
    0xff20c080,
    0xff21c7aa,
    0xff22ced5,
    0xff23d500,
    0xff24dc2a,
    0xff25e355,
    0xff27ea80,
    0xff28f1aa,
    0xff29f8d5,
    0xff2a0000,
    0xff2b072a,
    0xff2d0e55,
    0xff2e1580,
    0xff2f1caa,
    0xff3023d5,
    0xff312a00,
    0xff32312a,
    0xff343855,
    0xff354080,
    0xff3647aa,
    0xff374ed5,
    0xff385500,
    0xff3a5c2a,
    0xff3b6355,
    0xff3c6a80,
    0xff3d71aa,
    0xff3e78d5,
    0xff408000,
    0xff41872a,
    0xff428e55,
    0xff439580,
    0xff449caa,
    0xff45a3d5,
    0xff47aa00,
    0xff48b12a,
    0xff49b855,
    0xff4ac080,
    0xff4bc7aa,
    0xff4dced5,
    0xff4ed500,
    0xff4fdc2a,
    0xff50e355,
    0xff51ea80,
    0xff52f1aa,
    0xff54f8d5,
    0xff550000,
    0xff56072a,
    0xff570e55,
    0xff581580,
    0xff5a1caa,
    0xff5b23d5,
    0xff5c2a00,
    0xff5d312a,
    0xff5e3855,
    0xff604080,
    0xff6147aa,
    0xff624ed5,
    0xff635500,
    0xff645c2a,
    0xff656355,
    0xff676a80,
    0xff6871aa,
    0xff6978d5,
    0xff6a8000,
    0xff6b872a,
    0xff6d8e55,
    0xff6e9580,
    0xff6f9caa,
    0xff70a3d5,
    0xff71aa00,
    0xff72b12a,
    0xff74b855,
    0xff75c080,
    0xff76c7aa,
    0xff77ced5,
    0xff78d500,
    0xff7adc2a,
    0xff7be355,
    0xff7cea80,
    0xff7df1aa,
    0xff7ef8d5,
    0xff800000,
    0xff81072a,
    0xff820e55,
    0xff831580,
    0xff841caa,
    0xff8523d5,
    0xff872a00,
    0xff88312a,
    0xff893855,
    0xff8a4080,
    0xff8b47aa,
    0xff8d4ed5,
    0xff8e5500,
    0xff8f5c2a,
    0xff906355,
    0xff916a80,
    0xff9271aa,
    0xff9478d5,
    0xff958000,
    0xff96872a,
    0xff978e55,
    0xff989580,
    0xff9a9caa,
    0xff9ba3d5,
    0xff9caa00,
    0xff9db12a,
    0xff9eb855,
    0xffa0c080,
    0xffa1c7aa,
    0xffa2ced5,
    0xffa3d500,
    0xffa4dc2a,
    0xffa5e355,
    0xffa7ea80,
    0xffa8f1aa,
    0xffa9f8d5,
    0xffaa0000,
    0xffab072a,
    0xffad0e55,
    0xffae1580,
    0xffaf1caa,
    0xffb023d5,
    0xffb12a00,
    0xffb2312a,
    0xffb43855,
    0xffb54080,
    0xffb647aa,
    0xffb74ed5,
    0xffb85500,
    0xffba5c2a,
    0xffbb6355,
    0xffbc6a80,
    0xffbd71aa,
    0xffbe78d5,
    0xffc08000,
    0xffc1872a,
    0xffc28e55,
    0xffc39580,
    0xffc49caa,
    0xffc5a3d5,
    0xffc7aa00,
    0xffc8b12a,
    0xffc9b855,
    0xffcac080,
    0xffcbc7aa,
    0xffcdced5,
    0xffced500,
    0xffcfdc2a,
    0xffd0e355,
    0xffd1ea80,
    0xffd2f1aa,
    0xffd4f8d5,
    0xffd50000,
    0xffd6072a,
    0xffd70e55,
    0xffd81580,
    0xffda1caa,
    0xffdb23d5,
    0xffdc2a00,
    0xffdd312a,
    0xffde3855,
    0xffe04080,
    0xffe147aa,
    0xffe24ed5,
    0xffe35500,
    0xffe45c2a,
    0xffe56355,
    0xffe76a80,
    0xffe871aa,
    0xffe978d5,
    0xffea8000,
    0xffeb872a,
    0xffed8e55,
    0xffee9580,
    0xffef9caa,
    0xfff0a3d5,
    0xfff1aa00,
    0xfff2b12a,
    0xfff4b855,
    0xfff5c080,
    0xfff6c7aa,
    0xfff7ced5,
    0xfff8d500,
    0xfffadc2a,
    0xfffbe355,
    0xfffcea80,
    0xfffdf1aa,
    0xfffef8d5,
    0xff080808,
    0xff121212,
    0xff1c1c1c,
    0xff262626,
    0xff303030,
    0xff3a3a3a,
    0xff444444,
    0xff4e4e4e,
    0xff585858,
    0xff626262,
    0xff6c6c6c,
    0xff767676,
    0xff808080,
    0xff8a8a8a,
    0xff949494,
    0xff9e9e9e,
    0xffa8a8a8,
    0xffb2b2b2,
    0xffbcbcbc,
    0xffc6c6c6,
    0xffd0d0d0,
    0xffdadada,
    0xffe4e4e4,
    0xffeeeeee
];

pub const DEFAULT_ATTR: ScreenCharAttr = ScreenCharAttr { fg: 7, bg: 0, extra: 0 };

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
        let (font, block_font) = if (framebuffer.height * 2) < framebuffer.width {
            (&font::FONT_9X8, &font::FONT_9X8)
        } else {
            (&font::FONT_9X16, &font::BLOCK_FONT_9X16)
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
        let cursor = Cursor { col: 0, row: 0, top: 0, bottom: font.height - 1 };
        let screen = Screen::new(columns, rows, cursor)?;
        let screen = Mutex::<Screen>::new(screen);

        let display = Display {
            obj: Arc::<DisplayObj>::new(DisplayObj {
                width: framebuffer.width,
                height: framebuffer.height,
                screen,
                cond: Condvar::new(),
                font,
                block_font
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

    pub fn insert_at_cursor(&mut self, chrs: &[u8], cursor_shift: Option<u32>) {
        let mut screen = self.obj.screen.lock().unwrap();
        screen.insert_at_cursor(chrs, cursor_shift);
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
    let mut chr = screen.char_at(col, row);
    let font = if chr.attr.extra == 9 && chr.chr >= 210 {
        // devc-con-framebuffer extension:
        // Alternate font 9 uses special block characters between 210 and 256.
        chr.chr -= 210;
        display.block_font
    } else {
        display.font
    };

    // Don't render past right side of the frame.
    let font_width = font.width as u64;
    let start_x = col as u64 * font_width;
    let start_y = row as u64 * font.height as u64;
    if (start_x + font_width) > framebuffer.width as u64 {
        return;
    }

    let is_cursor = (col == screen.cursor.col) && (row == screen.cursor.row);

    // Determine foreground and background colours.
    let mut fg = PALETTE[chr.attr.fg as usize];
    let mut bg = PALETTE[chr.attr.bg as usize];

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
                fg = PALETTE[chr.attr.bg as usize];
                bg = PALETTE[chr.attr.fg as usize];
            } else {
                // Outside block, use normal colours.
                fg = PALETTE[chr.attr.fg as usize];
                bg = PALETTE[chr.attr.fg as usize];
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
