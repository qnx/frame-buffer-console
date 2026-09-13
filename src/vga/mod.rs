mod framebuffer;
mod screen;
mod font;
mod libc_missing;

use framebuffer::FrameBuffer;
use screen::Screen;

use std::sync::{Mutex, Condvar};
use std::thread;

pub struct Vga {
}

impl Vga {
    pub fn new() -> Result<Vga, ()> {
        // Map the frame buffer.
        let framebuffer = FrameBuffer::new()?;

        // Construct the screen object.
        let screen = Screen::new(framebuffer.width, framebuffer.height, &font::FONT_8X8)?;
        let screen_lock = Mutex::<Screen>::new(screen);
        let screen_cond = Condvar::new();

        // Start the screen update thread.
        thread::spawn(|| { screen_update_thread(screen_lock, screen_cond); });
        Ok(Vga {})
    }
}

fn screen_update_thread(screen_lock: Mutex::<Screen>, screen_cond: Condvar) {
    let mut screen = screen_lock.lock().unwrap();
    loop {
        screen.update();
        screen = screen_cond.wait(screen).unwrap();
    }
}
