use hidapi::HidApi;

fn main() {
    let api = HidApi::new().expect("Failed to init hidapi");
    println!("Looking for all ROYUAN / NYXI devices (VID 0x3151)...");
    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151 {
            println!(
                "Found: VID: 0x{:04x}, PID: 0x{:04x}, Iface: {}, UsagePage: 0x{:04x}, Usage: 0x{:04x}, Path: {:?}",
                dev_info.vendor_id(),
                dev_info.product_id(),
                dev_info.interface_number(),
                dev_info.usage_page(),
                dev_info.usage(),
                dev_info.path()
            );
            println!("  Product: {:?}", dev_info.product_string());
        }
    }
    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151
            && (dev_info.product_id() == 0x4015 || dev_info.product_id() == 0x4011)
            && dev_info.usage_page() == 0xffff
            && dev_info.usage() == 2
        {
            println!("\nTesting vendor interface: {:?}", dev_info.path());
            match dev_info.open_device(&api) {
                Ok(dev) => {
                    println!("Successfully opened vendor interface!");

                    // Try FEA_CMD_GET_INFOR (143 = 0x8F)
                    let mut report = [0u8; 65];
                    report[1] = 0x8F; // FEA_CMD_GET_INFOR
                    if let Ok(_) = dev.send_feature_report(&report) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let mut resp = [0u8; 65];
                        if let Ok(n) = dev.get_feature_report(&mut resp) {
                            println!("GET_INFOR Response ({} bytes): {:02x?}", n, &resp[..n]);
                            if n >= 5 {
                                let id_le = u16::from_le_bytes([resp[1], resp[2]]);
                                let id_be = u16::from_be_bytes([resp[1], resp[2]]);
                                println!("Interpreted as ID: LE={}, BE={}", id_le, id_be);
                            }
                        }
                    }

                    // Try FEA_CMD_GET_REV (128 = 0x80)
                    let mut report_rev = [0u8; 65];
                    report_rev[1] = 0x80;
                    if let Ok(_) = dev.send_feature_report(&report_rev) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let mut resp = [0u8; 65];
                        if let Ok(n) = dev.get_feature_report(&mut resp) {
                            println!("GET_REV Response ({} bytes): {:02x?}", n, &resp[..n]);
                        }
                    }
                }
                Err(e) => println!("Failed to open device: {}", e),
            }
        }
    }
}
