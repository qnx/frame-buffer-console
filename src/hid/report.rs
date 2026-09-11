/// Handle report descriptors

use crate::hid::device::*;

pub struct Report {
    pub data: [u8; 256]
}

#[derive(Debug)]
enum ItemType {
    Main,
    Global,
    Local
}

#[derive(Debug)]
enum MainItemType {
    Input,
    Output,
    Collection,
    Feature,
    EndCollection,
    Reserved
}

#[derive(Debug)]
enum GlobalItemType {
    UsagePage,
    LogicalMinimum,
    LogicalMaximum,
    PhysicalMinimum,
    PhysicalMaximum,
    UnitExponent,
    Unit,
    ReportSize,
    ReportId,
    ReportCount,
    Push,
    Pop,
    Reserved
}

#[derive(Debug)]
enum LocalItemType {
    Usage,
    UsageMinimum,
    UsageMaximum,
    DesignatorIndex,
    DesignatorMinimum,
    DesignatorMaximum,
    StringIndex,
    StringMinimum,
    StringMaximum,
    Delimiter,
    Reserved
}

#[derive(Debug)]
enum CollectionType {
    Physical,
    Application,
    Logical,
    Report,
    NamedArray,
    UsageSwitch,
    UsageModifier,
    Reserved
}

pub fn parse(report: &Report, len: usize, device: &mut Device) -> Result<(), ()> {
    let mut pos = 0usize;
    loop {
        // Parse the first byte of the item.
        let item = report.data[pos];
        let item_size = match item & 0x3 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4
        };

        let item_type = match item & 0xc {
            0 => ItemType::Main,
            4 => ItemType::Global,
            8 => ItemType::Local,
            _ => { return Err(()) } // Reserved/long item
        };

        // Ensure that the item fits in the report's data.
        let next_pos = pos + 1 + item_size;
        if next_pos > len {
            return Err(());
        }

        // Parse individual item.
        let item = &report.data[pos..next_pos];
        println!("{:?}", item_type);
        match item_type {
            ItemType::Main => parse_main_item(item, next_pos - pos, device)?,
            ItemType::Global => parse_global_item(item, next_pos - pos, device)?,
            ItemType::Local => parse_local_item(item, next_pos - pos, device)?,
        }

        // Move to the next item.
        pos = next_pos;
        if pos == len {
            break;
        }
    }

    Ok(())
}

fn parse_main_item(item: &[u8], len: usize, _device: &mut Device) -> Result<(), ()> {
    let tag = item[0];
    let item_type = match tag & 0xfc {
        0x80 => MainItemType::Input,
        0x90 => MainItemType::Output,
        0xa0 => MainItemType::Collection,
        0xb0 => MainItemType::Feature,
        0xc0 => MainItemType::EndCollection,
        _ => MainItemType::Reserved
    };

    println!("{:?}", item_type);
    match item_type {
        MainItemType::Collection => parse_collection(item, len)?,
        MainItemType::Input => parse_input(item, len)?,
        _ => {}
    }

    Ok(())
}

fn parse_collection(item: &[u8], len: usize) -> Result<(), ()> {
    if len == 1 {
        return Err(());
    }

    let coll_type = match item[1] {
        0x00 => CollectionType::Physical,
        0x01 => CollectionType::Application,
        0x02 => CollectionType::Logical,
        0x03 => CollectionType::Report,
        0x04 => CollectionType::NamedArray,
        0x05 => CollectionType::UsageSwitch,
        0x06 => CollectionType::UsageModifier,
        _ => CollectionType::Reserved
    };

    println!("{:?}", coll_type);
    Ok(())
}

fn parse_input(_item: &[u8], len: usize) -> Result<(), ()> {
    if len == 1 {
        return Err(());
    }


    Ok(())
}

fn parse_global_item(item: &[u8], len: usize, _device: &mut Device) -> Result<(), ()> {
    let tag = item[0];
    let item_type = match tag & 0xf0 {
        0x00 => GlobalItemType::UsagePage,
        0x10 => GlobalItemType::LogicalMinimum,
        0x20 => GlobalItemType::LogicalMaximum,
        0x30 => GlobalItemType::PhysicalMinimum,
        0x40 => GlobalItemType::PhysicalMaximum,
        0x50 => GlobalItemType::UnitExponent,
        0x60 => GlobalItemType::Unit,
        0x70 => GlobalItemType::ReportSize,
        0x80 => GlobalItemType::ReportId,
        0x90 => GlobalItemType::ReportCount,
        0xa0 => GlobalItemType::Push,
        0xb0 => GlobalItemType::Pop,
        _ => GlobalItemType::Reserved
    };

    println!("{:?}", item_type);
    match item_type {
        GlobalItemType::UsagePage => parse_usage_page(item, len)?,
        GlobalItemType::ReportId => parse_report_id(item, len)?,
        _ => {}
    }

    Ok(())
}

fn parse_usage_page(_item: &[u8], len: usize) -> Result<(), ()> {
    if len == 1 {
        return Err(());
    }

    Ok(())
}

fn parse_report_id(_item: &[u8], len: usize) -> Result<(), ()> {
    if len == 1 {
        return Err(());
    }

    Ok(())
}

fn parse_local_item(item: &[u8], len: usize, device: &mut Device) -> Result<(), ()> {
    let tag = item[0];
    let item_type = match tag & 0xf0 {
        0x00 => LocalItemType::Usage,
        0x10 => LocalItemType::UsageMinimum,
        0x20 => LocalItemType::UsageMaximum,
        0x30 => LocalItemType::DesignatorIndex,
        0x40 => LocalItemType::DesignatorMinimum,
        0x50 => LocalItemType::DesignatorMaximum,
        0x70 => LocalItemType::StringIndex,
        0x80 => LocalItemType::StringMinimum,
        0x90 => LocalItemType::StringMaximum,
        0xa0 => LocalItemType::Delimiter,
        _ => LocalItemType::Reserved
    };

    println!("{:?}", item_type);
    match item_type {
        LocalItemType::Usage => parse_usage(item, len, device)?,
        _ => {}
    }

    Ok(())
}

fn parse_usage(item: &[u8], len: usize, device: &mut Device) -> Result<(), ()> {
    if len == 1 {
        return Err(());
    }

    match item[1] {
        0x01 => device.set_type(DeviceType::Mouse),
        0x02 => device.set_type(DeviceType::Mouse),
        0x06 => device.set_type(DeviceType::Keyboard),
        _ => {}
    }

    Ok(())
}
