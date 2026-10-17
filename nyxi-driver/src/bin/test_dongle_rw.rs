use hidapi::HidApi;
use std::time::Duration;

fn main() {
    let api = HidApi::new().expect("Failed to init hidapi");
    println!("Testing direct 2.4G dongle communication...");

    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151 && dev_info.product_id() == 0x4011 {
            let iface = dev_info.interface_number();
            let up = dev_info.usage_page();
            let u = dev_info.usage();
            
            println!("\n------------------------------------------------");
            println!("DEVICE: iface={}, up=0x{:04x}, u=0x{:04x}, path={:?}", iface, up, u, dev_info.path());

            let dev = match dev_info.open_device(&api) {
                Ok(d) => d,
                Err(e) => {
                    println!("  Cannot open: {}", e);
                    continue;
                }
            };

            // Test 1: Drain any pending input
            let mut buf = [0u8; 128];
            while let Ok(n) = dev.read_timeout(&mut buf, 50) {
                if n > 0 {
                    println!("  Drained {} bytes: {:02x?}", n, &buf[..n]);
                } else {
                    break;
                }
            }

            // Test 2: Send Query 0x80 (Get Version)
            // Try write with 0x06 prefix (Report ID 6)
            let mut q_write = [0u8; 65];
            q_write[0] = 0x06;
            q_write[1] = 0x80;
            let res = dev.write(&q_write);
            println!("  write([0x06, 0x80...]) -> {:?}", res);

            // Read response
            let mut resp = [0u8; 128];
            match dev.read_timeout(&mut resp, 500) {
                Ok(n) => println!("  Response to write: {} bytes: {:02x?}", n, &resp[..n]),
                Err(e) => println!("  Read error: {}", e),
            }

            // Test 3: Send Query 0x80 via send_feature_report with 0x00 prefix
            let mut q_feat = [0u8; 65];
            q_feat[0] = 0x00;
            q_feat[1] = 0x80;
            let res_f = dev.send_feature_report(&q_feat);
            println!("  send_feature_report([0x00, 0x80...]) -> {:?}", res_f);

            // Read response
            match dev.read_timeout(&mut resp, 500) {
                Ok(n) => println!("  Response to feature: {} bytes: {:02x?}", n, &resp[..n]),
                Err(e) => println!("  Read error: {}", e),
            }

            // Also check get_feature_report
            let mut feat_resp = [0u8; 65];
            match dev.get_feature_report(&mut feat_resp) {
                Ok(n) => println!("  get_feature_report: {} bytes: {:02x?}", n, &feat_resp[..n]),
                Err(e) => println!("  get_feature_report error: {}", e),
            }
        }
    }
}
