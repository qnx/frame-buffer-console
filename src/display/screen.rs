/// The logical contents of the screen

use crate::display::{Rect, ScreenChar};
use crate::display::font::Font;
use crate::display::escape::*;
use crate::display::cursor::*;

pub struct Screen {
    /// Screen contents (character and attrbitute for each position).
    buffer: Vec<ScreenChar>,
    /// Row representing the logical top of the buffer.
    /// Changes with scrolling.
    buffer_top_row: u32,
    /// Horizontal size in characters.
    pub columns: u32,
    /// Vertical size in characters.
    pub rows: u32,
    /// Cursor position and shape.
    pub cursor: Cursor,
    /// Region of updated contents.
    dirty: Option<Rect>,
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
            buffer_top_row: 0,
            columns,
            rows,
            dirty: Some(Rect { left: 0, top: 0, right: columns - 1, bottom: rows - 1 }),
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

    /// Get the character at the given column and row.
    pub fn char_at(&self, col: u32, row: u32) -> &ScreenChar {
        let row = (row + self.buffer_top_row) % self.rows;
        &self.buffer[((row * self.columns) + col) as usize]
    }

    /// Get the current dirty rectangle and reset to an empty one.
    pub fn get_rect(&mut self) -> Option<Rect> {
        let rect = self.dirty;
        self.dirty = None;
        rect
    }

    /// Mark a region as dirty.
    /// The region is combined with the existing dirty rectangle.
    fn set_dirty(&mut self, rect: &Rect) -> bool {
        let mut is_dirty = false;
        if let Some(mut dirty) = self.dirty {
            if rect.left < dirty.left {
                dirty.left = rect.left;
                is_dirty = true;
            }

            if rect.right > dirty.right {
                dirty.right = rect.right;
                is_dirty = true;
            }

            if rect.top < dirty.top {
                dirty.top = rect.top;
                is_dirty = true;
            }

            if rect.bottom > dirty.bottom {
                dirty.bottom = rect.bottom;
                is_dirty = true;
            }

            if is_dirty {
                self.dirty = Some(dirty);
            }
        } else {
            self.dirty = Some(*rect);
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
                0x08 => self.move_to(CursorDir::Back, 1, &mut insert),
                0x1b => self.maybe_escape(&mut insert, &mut iter),
                b'\r' => self.move_to(CursorDir::BOL, 1, &mut insert),
                _ => self.insert_char(chr, &mut insert)
            }
        }

        // Set new cursor position.
        self.check_scroll(&mut insert);
        self.cursor.col = insert.col;
        self.cursor.row = insert.row;
        insert.update_rect(insert.col, insert.row);

        // Update the dirty rectangle.
        self.set_dirty(&insert.rect);
    }

    fn insert_char(&mut self, chr: u8, insert: &mut Insert) {
        // Check for scrolling if inserting into an overflow row.
        self.check_scroll(insert);

        // Update the buffer with the given character.
        self.buffer[insert.bufidx] = ScreenChar { chr, attr: self.attr };
        insert.bufidx += 1;

        // Update bounding rectangle of this insertion.
        insert.update_rect(insert.col, insert.row);

        // Move the position one to the right, wrapping as necessary.
        insert.col += 1;
        if insert.col == self.columns {
            insert.row += 1;
        }
    }

    fn insert_newline(&mut self, insert: &mut Insert) {
        // Check for scrolling if inserting into an overflow row.
        self.check_scroll(insert);

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
    }

    fn bell(&mut self) {
        // Do nothing for now.
    }

    /// Handle a potential escape sequence.
    /// The function is called when the ESC character is encouneted in a stream.
    fn maybe_escape(&mut self, insert: &mut Insert, iter: &mut std::str::Chars<'_>) {
        // Parse the escape sequence.
        self.esc.parse(iter.clone());

        // Act on the result.
        match self.esc.result {
            EscapeResult::SGR(attr) => {
                if attr == 0 {
                    self.attr = self.default_attr;
                } else {
                    self.attr = attr;
                }
            }
            EscapeResult::CUB(n) => self.move_to(CursorDir::Back, n, insert),
            EscapeResult::CUF(n) => self.move_to(CursorDir::Forward, n, insert),
            EscapeResult::DCH(n) => self.delete(n, insert),
            _ => {}
        }

        // Advance the iterator to after the escape sequence.
        for _ in 0..self.esc.len {
            iter.next();
        }

        self.esc.reset();
    }

    /// Update the insertion position based on the given direction.
    fn move_to(&self, dir: CursorDir, _num: u32, insert: &mut Insert) {
        let (mut col, mut row, mut idx) = (insert.col, insert.row, insert.bufidx);
        match dir {
            CursorDir::Forward => {
                if col == self.columns - 1 {
                    if row < self.rows {
                        col = 0;
                        row += 1;
                        idx += 1;
                    }
                } else {
                    col += 1;
                    idx += 1;
                }
            }
            CursorDir::Back => {
                if col == 0 {
                    if row > 0 {
                        col = self.columns - 1;
                        row -= 1;
                        idx -= 1;
                    }
                } else {
                    col -= 1;
                    idx -= 1;
                }
            }
            CursorDir::BOL => {
                idx -= col as usize;
                col = 0;
            }
        };

        insert.col = col;
        insert.row = row;
        insert.bufidx = idx;
    }

    /// Delete num characters from the current position.
    fn delete(&mut self, num: u32, insert: &mut Insert) {
        for i in 0..num as usize {
            if (insert.bufidx + i) == self.buffer.len() {
                break;
            }

            self.buffer[insert.bufidx + i] = BLANK_CHAR;
        }
    }

    fn check_scroll(&mut self, insert: &mut Insert) {
        // Check if scolling is needed.
        if insert.row < self.rows {
            return;
        }

        // Clear the line to become the new bottom row.
        assert_eq!(insert.col, 0);
        insert.bufidx = (self.buffer_top_row * self.columns) as usize;
        for i in 0..self.columns as usize {
            self.buffer[insert.bufidx + i] = BLANK_CHAR;
        }

        // Now inserting into the new bottom row.
        insert.row -= 1;

        // Rotate the screen buffer.
        self.buffer_top_row = (self.buffer_top_row + 1) % self.rows;

        // The entire screen needs to be updated.
        self.dirty =
            Some(Rect { left: 0, top: 0, right: self.columns - 1, bottom: self.rows - 1});
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
}

impl Insert {
    pub fn new(screen: &Screen, col: u32, row: u32) -> Self {
        let buf_row = (screen.buffer_top_row + row) % screen.rows;
        Insert {
            col,
            row,
            bufidx: (buf_row * screen.columns + col) as usize,
            rect: Rect { left: col, right: col, top: row, bottom: row }
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
