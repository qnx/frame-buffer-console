/// Handle report descriptors

pub struct Report {
    pub data: [u8; 256]
}

#[derive(Debug)]
enum ItemType {
    Main,
    Global,
    Local,
    Reserved,
    Long
}

pub fn parse(report: &Report, len: usize) {
    let mut pos = 0usize;
    loop {
        // Parse the first byte of the item.
        let item = report.data[pos];
        let item_size;
        let item_type;

        if item == 0xe {
            // Long item.
            // Size is on next byte.
            if pos + 1 == len {
                println!("Bad report");
                break;
            }

            item_size = report.data[pos + 1] as usize;
            item_type = ItemType::Long;
        } else {
            // Short item.
            // Size and type are in the first byte.
            item_size = match item & 0x3 {
                0 => 0,
                1 => 1,
                2 => 2,
                _ => 4
            };

            item_type = match item & 0xc {
                0 => ItemType::Main,
                4 => ItemType::Global,
                8 => ItemType::Local,
                _ => ItemType::Reserved
            };
        }

        println!("Found item {:x} {:?} {item_size}", item, item_type);

        // Ensure that the item fits in the report's data.
        let next_pos = pos + 1 + item_size;
        if next_pos > len {
            println!("Bad report");
            break;
        }

        match item_type {
            ItemType::Main => parse_main_item(&report.data[pos..next_pos]),
            _ => {}
        }

        pos = next_pos;
        if pos == len {
            println!("Finished report");
            break;
        }

    }
}

fn parse_main_item(item: &[u8]) {
    let tag = item[0];
    match tag & 0xfc {
        0x80 => println!("Input"),
        0x90 => println!("Output"),
        0xa0 => println!("Collection"),
        0xb0 => println!("Feature"),
        0xc0 => println!("End Collection"),
        _ => println!("Reserved")
    };
}
