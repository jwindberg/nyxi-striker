use hidapi::HidApi;
use std::time::Duration;

fn main() {
    println!("========================================================");
    println!("          NYXI CA01 KEYBOARD: READ API DIAGNOSTIC       ");
    println!("========================================================");

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to init hidapi: {}", e);
            return;
        }
    };

    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151 && dev_info.product_id() == 0x4015 {
            let iface = dev_info.interface_number();
            let usage = dev_info.usage();
            let up = dev_info.usage_page();
            let path = dev_info.path();

            println!("\n--------------------------------------------------------");
            println!("Testing Device: iface={}, usage=0x{:04x}, up=0x{:04x}", iface, usage, up);
            println!("Path: {:?}", path);

            let dev = match dev_info.open_device(&api) {
                Ok(d) => d,
                Err(e) => {
                    println!("  Cannot open device: {}", e);
                    continue;
                }
            };

            // 1. USB String Descriptors
            match dev.get_manufacturer_string() {
                Ok(Some(s)) => println!("  Manufacturer: '{}'", s),
                Ok(None) => println!("  Manufacturer: None"),
                Err(e) => println!("  Manufacturer Err: {}", e),
            }
            match dev.get_product_string() {
                Ok(Some(s)) => println!("  Product:      '{}'", s),
                Ok(None) => println!("  Product: None"),
                Err(e) => println!("  Product Err: {}", e),
            }
            match dev.get_serial_number_string() {
                Ok(Some(s)) => println!("  Serial:       '{}'", s),
                Ok(None) => println!("  Serial: None"),
                Err(e) => println!("  Serial Err: {}", e),
            }

            // Check indexed strings 1 to 5
            for idx in 1..=5 {
                if let Ok(Some(s)) = dev.get_indexed_string(idx) {
                    println!("  Indexed String {}: '{}'", idx, s);
                }
            }

            // 2. Feature Report Reads for Report IDs 0, 1, 2, 3, 5, 6
            for rpt_id in [0u8, 1, 2, 3, 4, 5, 6, 7, 0x80, 0x84, 0x87, 0x8F] {
                let mut buf = [0u8; 65];
                buf[0] = rpt_id;
                match dev.get_feature_report(&mut buf) {
                    Ok(n) => {
                        let non_zero = buf[..n].iter().any(|&b| b != 0);
                        if non_zero {
                            println!("  [!] get_feature_report(ID={:02x}) -> {} bytes NON-ZERO: {:02x?}", rpt_id, n, &buf[..n.min(20)]);
                        } else {
                            println!("  get_feature_report(ID={:02x}) -> {} bytes (all zeroes)", rpt_id, n);
                        }
                    }
                    Err(e) => {
                        // Suppress verbose errors if unsupported report id
                        println!("  get_feature_report(ID={:02x}) -> Err: {}", rpt_id, e);
                    }
                }
            }

            // 3. Interrupt IN Read (read_timeout)
            let mut in_buf = [0u8; 64];
            match dev.read_timeout(&mut in_buf, 50) {
                Ok(n) if n > 0 => {
                    println!("  [!] read_timeout -> {} bytes: {:02x?}", n, &in_buf[..n]);
                }
                Ok(_) => {
                    println!("  read_timeout -> 0 bytes (no pending interrupt report)");
                }
                Err(e) => {
                    println!("  read_timeout -> Err: {}", e);
                }
            }
        }
    }

    println!("\n========================================================");
    println!("Diagnostic complete.");
    println!("========================================================");
}
