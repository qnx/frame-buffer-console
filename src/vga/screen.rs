/// The logical contents of the screen

use crate::vga::font::Font;

pub struct Screen {
    buffer: Vec<ScreenChar>,
    columns: u32,
    rows: u32
}

/// A character on the screen.
/// Every position on the screen is represented by two bytes, the character and attributes (colour,
/// background, etc.).
#[derive(Clone)]
struct ScreenChar {
    chr: u8,
    attr: u8
}

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

        // FIXME:
        // Would like to report allocation error rather than panic.
        screen.buffer.resize((columns * rows) as usize, ScreenChar { chr: 0, attr: 0 });
        Ok(screen)
    }
}
