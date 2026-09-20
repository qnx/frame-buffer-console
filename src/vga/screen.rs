/// The logical contents of the screen

use crate::vga::{Rect, ScreenChar, Cursor};
use crate::vga::font::Font;
use crate::vga::escape::Escape;

pub struct Screen {
    /// Screen contents (character and attrbitute for each position).
    pub buffer: Vec<ScreenChar>,
    /// Horizontal size in characters.
    pub columns: u32,
    /// Vertical size in characters.
    pub rows: u32,
    /// Cursor position and shape.
    pub cursor: Cursor,
    /// Region of updated contents.
    dirty: Rect,
    /// Default character attributes.
    default_attr: u8,
    /// Current attributes.
    attr: u8,
    /// Escape sequence parser.
    esc: Escape
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
            cursor: Cursor { col: 0, row: 0, top: 6, bottom: 6 },
            default_attr: 0x07,
            attr: 0x07,
            esc: Escape::new()
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

    /// Mark a region as dirty.
    /// The region is combined with the existing dirty rectangle.
    fn set_dirty(&mut self, rect: &Rect) -> bool {
        let mut is_dirty = false;

        if rect.left < self.dirty.left {
            self.dirty.left = rect.left;
            is_dirty = true;
        }

        if rect.right > self.dirty.right {
            self.dirty.right = rect.right;
            is_dirty = true;
        }

        if rect.top < self.dirty.top {
            self.dirty.top = rect.top;
            is_dirty = true;
        }

        if rect.bottom > self.dirty.bottom {
            self.dirty.bottom = rect.bottom;
            is_dirty = true;
        }

        is_dirty
    }

    pub fn insert_at_cursor(&mut self, chrs: &str) {
        // Mark starting position.
        let mut insert = Insert::new(self, self.cursor.col, self.cursor.row);
        let mut iter = chrs.chars();

        // Resume parsing an escape sequence from a previous call.
        if self.esc.is_active() {
            self.maybe_escape(&mut insert, &mut iter);
        }

        // Insert characters.
        while let Some(chr) = iter.next() {
            let chr = chr as u8;
            match chr {
                b'\n' => self.insert_newline(&mut insert),
                0x07 => self.bell(),
                0x08 => self.backspace(&mut insert),
                0x1b => self.maybe_escape(&mut insert, &mut iter),
                _ => self.insert_char(chr, &mut insert)
            }
        }

        // Set new cursor position.
        self.cursor.col = insert.col;
        self.cursor.row = insert.row;
        insert.update_rect(insert.col, insert.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);

        // FIXME:
        // Scrolling.
    }

    fn insert_char(&mut self, chr: u8, insert: &mut Insert) {
        // Update the buffer with the given character.
        self.buffer[insert.bufidx] = ScreenChar { chr, attr: self.attr };
        insert.bufidx += 1;

        // Update bounding rectangle of this insertion.
        insert.update_rect(insert.col, insert.row);

        // Move the position one to the right, wrapping as necessary.
        insert.col += 1;
        if insert.col == self.columns {
            insert.col = 0;
            insert.row += 1;
            if insert.row == self.rows {
                insert.scroll += 1;
            }
        }
    }

    fn insert_newline(&mut self, insert: &mut Insert) {
        // Fill the buffer with empty characters from the given position to the end of the row.
        for _ in insert.col..self.columns {
            self.buffer[insert.bufidx] = BLANK_CHAR;
            insert.bufidx += 1;
        }

        // Update bounding rectangle.
        insert.update_rect(self.columns - 1, insert.row);

        // Move to the next line.
        insert.col = 0;
        insert.row += 1;
        if insert.row == self.rows {
            insert.scroll += 1;
        }

    }

    fn backspace(&mut self, insert: &mut Insert) {
        // Move the insertion position backwards.
        if insert.col > 0 {
            insert.col -= 1;
        } else if insert.row > 0 {
            insert.col = 0;
            insert.row -= 1;
        } else {
            // At the beginning of the screen, cannot go backwards.
            return;
        }

        insert.bufidx -= 1;
        self.buffer[insert.bufidx] = BLANK_CHAR;
    }

    fn bell(&mut self) {
        // Do nothing for now.
    }

    /// Handle a potential escape sequence.
    /// The function is called when the ESC character is encouneted in a stream.
    fn maybe_escape(&mut self, insert: &mut Insert, iter: &mut std::str::Chars<'_>) {
        self.esc.parse(iter.clone());

        // Update attributes, if required.
        if let Some(attr) = self.esc.attr {
            if attr == 0 {
                self.attr = self.default_attr;
            } else {
                self.attr = attr;
            }
        }

        // Insert a character, if required.
        if let Some(chr) = self.esc.chr {
            self.insert_char(chr, insert);
        }

        // Advance the iterator to after the escape sequence.
        for _ in 0..self.esc.len {
            iter.next();
        }

        self.esc.reset();
    }
}

/// Tracks single- and multi-characters insertions into the screen's buffer.
struct Insert {
    /// Column for the next character.
    col: u32,
    /// Row for the next character.
    row: u32,
    /// Position in the screen buffer.
    bufidx: usize,
    /// Area of the screen affected by the update.
    rect: Rect,
    /// How many lines of scrolling are needed.
    scroll: u32
}

impl Insert {
    pub fn new(screen: &Screen, col: u32, row: u32) -> Self {
        Insert {
            col,
            row,
            bufidx: (row * screen.columns + col) as usize,
            rect: Rect { left: col, right: col, top: row, bottom: row },
            scroll: 0
        }
    }

    pub fn update_rect(&mut self, col: u32, row: u32) {
        if col < self.rect.left {
            // Can happen on a new line.
            self.rect.left = col;
        }

        if col > self.rect.right {
            self.rect.right = col;
        }

        if row > self.rect.bottom {
            self.rect.bottom = row;
        }
    }
}
