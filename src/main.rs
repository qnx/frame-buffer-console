mod hid;

use std::thread;
use std::time::Duration;
use hid::Hid;

fn main() {
    let hid = Hid::new().unwrap();
    thread::sleep(Duration::from_secs(1));
    drop(hid);
}
