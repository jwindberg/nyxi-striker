use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let api = HidApi::new().expect("Failed to init hidapi");
    println!("Testing GET_LEDPARAM (0x87) on NYXI CA01 (0x3151:0x4015)...");

    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151 && dev_info.product_id() == 0x4015 {
            let iface = dev_info.interface_number();
            let usage = dev_info.usage();
            let usage_page = dev_info.usage_page();
            println!("\n=======================================================");
            println!("DEVICE: iface={}, usage=0x{:04x}, up=0x{:04x}, path={:?}", iface, usage, usage_page, dev_info.path());

            let dev = match dev_info.open_device(&api) {
                Ok(d) => d,
                Err(e) => {
                    println!("Cannot open: {}", e);
                    continue;
                }
            };

            // Prepare 0x87 query
            let mut req = [0u8; 65];
            req[1] = 0x87; // FEA_CMD_GET_LEDPARAM

            // Try sending feature report
            match dev.send_feature_report(&req) {
                Ok(_) => println!("send_feature_report(0x87) -> OK"),
                Err(e) => println!("send_feature_report(0x87) -> ERR: {}", e),
            }

            sleep(Duration::from_millis(100));

            let mut resp = [0u8; 65];
            match dev.get_feature_report(&mut resp) {
                Ok(n) => {
                    println!("get_feature_report -> {} bytes: {:02x?}", n, &resp[..n]);
                }
                Err(e) => println!("get_feature_report -> ERR: {}", e),
            }

            // Also try with read_timeout
            let mut buf = [0u8; 64];
            match dev.read_timeout(&mut buf, 200) {
                Ok(n) => println!("read_timeout -> {} bytes: {:02x?}", n, &buf[..n]),
                Err(e) => println!("read_timeout -> ERR: {}", e),
            }
        }
    }
}
