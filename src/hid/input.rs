/// Input report parser

use crate::hid::*;

#[derive(Debug)]
pub struct InputReport {
    data: Vec<u8>
}

impl InputReport {
    pub fn new(event: &HidEvent, device: &Device) -> Result<InputReport, ()> {
        // Extract the data.
        const HID_EVENT_SIZE: usize = size_of::<HidEvent>() - MAX_REPORT_LEN;
        let event_length = event.length as usize;
        if event_length < HID_EVENT_SIZE {
            println!("Bad input report");
            return Err(());
        }

        let data_offset = event_length - HID_EVENT_SIZE;

        let data_len = event.dlen as usize;
        if data_len != device.report_len as usize {
            println!("Bad input report");
            return Err(());
        }

        if data_len + data_offset > MAX_REPORT_LEN {
            println!("Bad input report");
            return Err(());
        }

        let report_data = &event.report[data_offset..data_offset + data_len];
        Ok(InputReport { data: report_data.to_vec() })
    }
}
