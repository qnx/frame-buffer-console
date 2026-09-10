mod hid;

use hid::Hid;

fn main() {
    let _fd = Hid::new().unwrap();
}
