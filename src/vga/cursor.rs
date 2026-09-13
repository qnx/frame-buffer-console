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
