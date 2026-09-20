mod framebuffer;
mod screen;
mod font;
mod cursor;
mod escape;

use framebuffer::FrameBuffer;
use screen::Screen;
use font::Font;
use cursor::{Cursor};

use std::sync::{Arc, Mutex, Condvar};
use std::thread;

pub struct Vga {
    obj: Arc<VgaObj>
}

struct VgaObj {
    framebuffer: FrameBuffer,
    screen: Mutex<Screen>,
    cond: Condvar,
    font: &'static Font
}

/// A character on the screen.
/// Every position on the screen is represented by two bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone, Debug)]
pub struct ScreenChar {
    pub chr: u8,
    pub attr: u8
}

const PALETTE: [u32; 16] = [
    0xFF000000, // 0 - black
    0xFF0000AA, // 1 - blue
    0xFF00AA00, // 2 - green
    0xFF00AAAA, // 3 - cyan
    0xFFAA0000, // 4 - red
    0xFFAA00AA, // 5 - magenta
    0xFFAA5500, // 6 - brown
    0xFFAAAAAA, // 7 - white
    0xFF555555, // 8 - grey
    0xFF5555FF, // 9 - light blue
    0xFF55FF55, // 10 - light green
    0xFF55FFFF, // 11 - light cyan
    0xFFFF5555, // 12 - light red
    0xFFFF55FF, // 13 - light magenta
    0xFFFFFF55, // 14 - yellow
    0xFFFFFFFF, // 15 - bright white
];

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32
}

impl Vga {
    pub fn new() -> Result<Vga, ()> {
        // Map the frame buffer.
        let framebuffer = FrameBuffer::new()?;

        // Construct the screen object.
        let font = &font::FONT_8X8;
        let screen = Screen::new(framebuffer.width, framebuffer.height, font)?;
        let screen = Mutex::<Screen>::new(screen);

        let vga = Vga {
            obj: Arc::<VgaObj>::new(VgaObj {
                framebuffer,
                screen,
                cond: Condvar::new(),
                font
            })
        };

        // Start the screen update thread.
        let cloned_vga = vga.obj.clone();
        thread::spawn(move || { screen_update_thread(cloned_vga); });
        Ok(vga)
    }

    pub fn clone(&self) -> Vga {
        Vga { obj: self.obj.clone() }
    }

    pub fn insert_at_cursor(&mut self, chrs: &str) {
        let mut screen = self.obj.screen.lock().unwrap();
        screen.insert_at_cursor(chrs);
        self.obj.cond.notify_one();
    }

    pub fn screen_size(&self) -> (u32, u32) {
        let screen = self.obj.screen.lock().unwrap();
        (screen.columns, screen.rows)
    }

    pub fn framebuffer_size(&self) -> (u32, u32) {
        (self.obj.framebuffer.width, self.obj.framebuffer.height)
    }
}

fn screen_update_thread(vga: Arc<VgaObj>) {
    let mut screen = vga.screen.lock().unwrap();
    loop {
        // Determine size of rectangle to update. If it is first or last row or column, update
        // to the edge of the screen.
        if let Some(rect) = screen.get_rect() {
            draw_screen(&vga, &screen, &rect);
        }

        // Wait for a signal to redraw the screen.
        screen = vga.cond.wait(screen).unwrap();
    }
}

fn draw_screen(vga: &VgaObj, screen: &Screen, rect: &Rect) {
    for row in rect.top..=rect.bottom {
        for col in rect.left..=rect.right {
            draw_glyph(&vga, screen, col, row);
        }
    }
}

/// Draw the glyph of a given character into the frame buffer at the specified column and row.
fn draw_glyph(vga: &VgaObj, screen: &Screen, col: u32, row: u32) {
    const BYTES_PER_PIXEL: u64 = size_of::<u32>() as u64;

    let font = vga.font;
    let framebuffer = &vga.framebuffer;
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
    let mut bg = PALETTE[((chr.attr >> 4) & 0x7) as usize];

    // Find the starting pixel of the glyph in the frame buffer.
    let stride = framebuffer.stride as u64;
    let mut pixel = framebuffer.addr + (start_y * stride);

    // Draw the glyph itself.
    let mut bitidx = chr.chr as u32 * font.wbits as u32 * font.hbits as u32;
    for i in 0..font.hbits {
        let mut xoff = start_x * BYTES_PER_PIXEL;

        if is_cursor {
            // Drawing glyph in the cursor position.
            if (i >= screen.cursor.top) && (i <= screen.cursor.bottom) {
                // Inside block, use inverted colours.
                fg = PALETTE[((chr.attr >> 4) & 0xf) as usize];
                bg = PALETTE[(chr.attr & 0x7) as usize];
            } else {
                // Outside block, use normal colours.
                fg = PALETTE[(chr.attr & 0xf) as usize];
                bg = PALETTE[((chr.attr >> 4) & 0x7) as usize];
            }
        }

        // Draw glyph line.
        for _ in 0..font.wbits {
            let pixel_colour = match is_foreground(font, bitidx) {
                true => fg,
                false => bg
            };

            unsafe { *((pixel + xoff) as *mut u32) = pixel_colour };

            xoff += BYTES_PER_PIXEL;
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

            unsafe { *((pixel + xoff) as *mut u32) = pixel_colour };
            xoff += BYTES_PER_PIXEL;
        }

        pixel += stride as u64;
    }
}

fn is_foreground(font: &Font, bitidx: u32) -> bool {
    (font.bitmap[(bitidx / 8) as usize] & (1 << (7 - (bitidx % 8)))) != 0
}
