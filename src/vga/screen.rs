/// The logical contents of the screen

use crate::vga::font::Font;

#[derive(Debug)]
pub struct Screen {
    buffer: Vec<ScreenChar>,
    columns: u32,
    rows: u32
}

/// A character on the screen.
/// Every position on the screen is represented by two bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone, Debug)]
struct ScreenChar {
    chr: u8,
    attr: u8
}

const BLANK_CHAR: ScreenChar = ScreenChar {
    chr: ' ' as u8,
    attr: 0x17
};

impl Screen {
    pub fn new(width: u32, height: u32, font: &Font) -> Result<Screen, ()> {
        // Calculate the number of columns and rows.
        let columns = width / font.width;
        let rows = height / font.height;
        let mut screen = Screen {
            buffer: Vec::<ScreenChar>::new(),
            columns,
            rows
        };

        // Start with a clear screen.
        // FIXME:
        // Would like to report allocation error rather than panic.
        screen.buffer.resize((columns * rows) as usize, BLANK_CHAR);
        Ok(screen)
    }

    pub fn update(&self) {
    }
}
