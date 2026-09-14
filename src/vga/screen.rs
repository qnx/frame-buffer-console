/// The logical contents of the screen

use crate::vga::font::Font;
use crate::vga::{Rect, ScreenChar, Cursor, CursorDir};

pub struct Screen {
    pub buffer: Vec<ScreenChar>,
    pub columns: u32,
    pub rows: u32,
    pub cursor: Cursor,
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
            cursor: Cursor { col: 0, row: 0, top: 6, bottom: 6 }
        };

        // Start with a clear screen.
        // FIXME:
        // Would like to report allocation error rather than panic.
        screen.buffer.resize((columns * rows) as usize, BLANK_CHAR);
        Ok(screen)
    }

    pub fn set_char(&mut self, chr: ScreenChar, col: u32, row: u32) {
        if col >= self.columns || row > self.rows {
            return;
        }

        let pos = (row * self.columns + col) as usize;
        self.buffer[pos] = chr;
    }

    /// Get the current dirty rectangle and reset to an empty one.
    pub fn get_rect(&mut self) -> Rect {
        let rect = self.dirty;
        self.dirty = Rect { left: self.columns, top: self.rows, right: 0, bottom: 0 };
        rect
    }

    /// Mark a region as dirty.
    /// The region is combined with the existing dirty rectangle.
    pub fn set_dirty(&mut self, col: u32, row: u32, nchars: u32) -> bool {
        let mut is_dirty = false;

        if col + nchars > self.columns {
            // Multi-line update.
            // Extend the rectangle horizontally to include complete rows.
            if self.dirty.left > 0 {
                self.dirty.left = 0;
                is_dirty = true;
            }

            if self.dirty.right < self.columns - 1 {
                self.dirty.right = self.columns - 1;
                is_dirty = true;
            }

            // Calculate the rows to include.
            if row < self.dirty.top {
                self.dirty.top = row;
                is_dirty = true;
            }

            let mut bottom = row + (col + nchars) / self.columns;
            if bottom >= self.rows {
                bottom = self.rows - 1;
            }

            if bottom > self.dirty.bottom {
                self.dirty.bottom = bottom;
                is_dirty = true;
            }
        } else {
            // Single line update.
            if col < self.dirty.left {
                self.dirty.left = col;
                is_dirty = true;
            }

            if row < self.dirty.top {
                self.dirty.top = row;
                is_dirty = true;
            }

            // We already know that col + nchars - 1 < self.columns.
            let right = col + nchars - 1;
            if right > self.dirty.right {
                self.dirty.right = right;
                is_dirty = true;
            }

            if row > self.dirty.bottom {
                self.dirty.bottom = row;
                is_dirty = true;
            }
        }

        is_dirty
    }

    /// Reposition cursor according to the direction.
    pub fn move_cursor(&mut self, dir: CursorDir) {
        let (mut col, mut row) = (self.cursor.col, self.cursor.row);
        match dir {
            CursorDir::_Left => {
                if col > 0 {
                    col -= 1
                }
            },
            CursorDir::_Right => {
                if col < self.columns - 1 {
                    col += 1
                }
            },
            CursorDir::_Up => {
                if row > 0 {
                    row -= 1
                }
            }
            CursorDir::_Down => {
                if row < self.rows - 1 {
                    row += 1
                }
            }
            CursorDir::Forward => {
                if col < self.columns - 1 {
                    col += 1
                } else if row < self.rows - 1 {
                    col = 0;
                    row += 1;
                }
            },
            CursorDir::_Back => {
                if col > 0 {
                    col -= 1;
                } else if row > 0 {
                    col = self.columns - 1;
                    row -= 1;
                }
            },
        }

        self.cursor.col = col;
        self.cursor.row = row;
    }
}
