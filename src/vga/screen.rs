/// The logical contents of the screen

use crate::vga::font::Font;
use crate::vga::{Rect, ScreenChar};

#[derive(Debug)]
pub struct Screen {
    pub buffer: Vec<ScreenChar>,
    pub columns: u32,
    pub rows: u32,
    dirty: Rect
}

const BLANK_CHAR: ScreenChar = ScreenChar {
    chr: ' ' as u8,
    attr: 0x07
};

impl Screen {
    pub fn new(width: u32, height: u32, font: &Font) -> Result<Screen, ()> {
        // Calculate the number of columns and rows.
        let columns = width / font.width as u32;
        let rows = height / font.height as u32;
        let mut screen = Screen {
            buffer: Vec::<ScreenChar>::new(),
            columns,
            rows,
            dirty: Rect { left: 0, top: 0, right: columns, bottom: rows },
        };

        // Start with a clear screen.
        // FIXME:
        // Would like to report allocation error rather than panic.
        screen.buffer.resize((columns * rows) as usize, BLANK_CHAR);
        Ok(screen)
    }

    /// Get the current dirty rectangle and reset to an empty one.
    pub fn get_rect(&mut self) -> Rect {
        let rect = self.dirty;
        self.dirty = Rect { left: self.columns, top: self.rows, right: 0, bottom: 0 };
        rect
    }

    pub fn set_char(&mut self, chr: ScreenChar, col: u32, row: u32) {
        if col >= self.columns || row > self.rows {
            return;
        }

        let pos = (row * self.columns + col) as usize;
        self.buffer[pos] = chr;
        if col < self.dirty.left {
            self.dirty.left = col;
        }
        if col > self.dirty.right {
            self.dirty.right = col;
        }
        if row < self.dirty.top {
            self.dirty.top = row;
        }
        if row > self.dirty.bottom {
            self.dirty.bottom = row;
        }
    }
}
