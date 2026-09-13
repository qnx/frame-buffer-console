/// The logical contents of the screen

use crate::vga::font::Font;
use crate::vga::Rect;

#[derive(Debug)]
pub struct Screen {
    pub buffer: Vec<ScreenChar>,
    pub columns: u32,
    pub rows: u32,
    dirty: Rect
}

/// A character on the screen.
/// Every position on the screen is represented by two bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone, Debug)]
pub struct ScreenChar {
    pub chr: u8,
    pub attr: u8
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
}
