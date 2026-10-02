/// Screen cursor

pub struct Cursor {
    /// Cursor horizontal position.
    pub col: u32,
    /// Cursor vertical position.
    pub row: u32,
    /// First visible line.
    pub top: u8,
    /// Last visible line.
    pub bottom: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum CursorDir {
    /// Right with wrap to the next line.
    Forward(u32),
    /// Left with wrap to the previous line.
    Back(u32),
    /// Beginning of line.
    BOL
}
