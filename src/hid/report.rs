/// Report parser

// FIXME:
// Incomplete. Need to determine exactly which items belong to the keyboard input that is of
// interest. Right now the last input item will override previous ones, which may not be correct.

use crate::hid::device::*;

/// Parsed report structure.
/// The public fields carry the parsed information, which private fields hold global state used
/// during parsing.
/// The console is only interested in input devices.
pub struct Report {
    pub device_type: DeviceType,
    pub input_id: u8,
    pub input_report_len: u8,
    report_id: u8,
    report_size: u8,
    verbose: bool
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

impl Report {
    pub fn new(data: &[u8], len: usize, verbose: bool) -> Result<Report, ()> {
        let mut report = Report {
            device_type: DeviceType::Other,
            input_id: 0,
            input_report_len: 0,
            report_id: 0,
            report_size: 0,
            verbose
        };

        report.parse(data, len)?;
        Ok(report)
    }

    fn parse(&mut self, data: &[u8], len: usize) -> Result<(), ()> {
        let mut pos = 0usize;
        loop {
            // Parse the first byte of the item.
            let item = data[pos];
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
            let item = &data[pos..next_pos];
            if self.verbose { println!("{:x?} {:?}", item, item_type); }
            match item_type {
                ItemType::Main => self.main_item(item, next_pos - pos)?,
                ItemType::Global => self.global_item(item, next_pos - pos)?,
                ItemType::Local => self.local_item(item, next_pos - pos)?,
            }

            // Move to the next item.
            pos = next_pos;
            if pos == len {
                break;
            }
        }

        Ok(())
    }

    fn main_item(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
        let tag = item[0];
        let item_type = match tag & 0xfc {
            0x80 => MainItemType::Input,
            0x90 => MainItemType::Output,
            0xa0 => MainItemType::Collection,
            0xb0 => MainItemType::Feature,
            0xc0 => MainItemType::EndCollection,
            _ => MainItemType::Reserved
        };

        if self.verbose { println!("{:?}", item_type); }
        match item_type {
            MainItemType::Collection => self.collection(item, len)?,
            MainItemType::Input => self.input(item, len)?,
            _ => {}
        }

        Ok(())
    }

    fn collection(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
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

        if self.verbose { println!("{:?}", coll_type); }
        Ok(())
    }

    fn input(&mut self ,_item: &[u8], len: usize) -> Result<(), ()> {
        if len == 1 {
            return Err(());
        }

        self.input_id = self.report_id;
        self.input_report_len = self.report_size;
        Ok(())
    }

    fn global_item(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
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

        if self.verbose { println!("{:?}", item_type); }
        match item_type {
            GlobalItemType::UsagePage => self.usage_page(item, len)?,
            GlobalItemType::ReportId => self.report_id(item, len)?,
            GlobalItemType::ReportSize => self.report_size(item, len)?,
            _ => {}
        }

        Ok(())
    }

    fn usage_page(&mut self, _item: &[u8], len: usize) -> Result<(), ()> {
        if len == 1 {
            return Err(());
        }

        Ok(())
    }

    fn report_id(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
        if len == 1 {
            return Err(());
        }

        self.report_id = item[1];
        Ok(())
    }

    fn report_size(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
        if len == 1 {
            return Err(());
        }

        self.report_size = item[1];
        Ok(())
    }

    fn local_item(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
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

        if self.verbose { println!("{:?}", item_type); }
        match item_type {
        LocalItemType::Usage => self.usage(item, len)?,
            _ => {}
        }

        Ok(())
    }

    fn usage(&mut self, item: &[u8], len: usize) -> Result<(), ()> {
        if len == 1 {
            return Err(());
        }

        match item[1] {
            0x01 => self.device_type = DeviceType::Mouse,
            0x02 => self.device_type = DeviceType::Mouse,
            0x06 => self.device_type = DeviceType::Keyboard,
            _ => {}
        }

        Ok(())
    }
}
