use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;
use std::io::{self, Write};

fn countdown(sec: u32, msg: &str) {
    println!("\n>>> {}", msg);
    for i in (1..=sec).rev() {
        print!("    Holding for {}s... \r", i);
        let _ = io::stdout().flush();
        sleep(Duration::from_secs(1));
    }
    println!("    Holding for 0s... DONE.");
}

fn main() {
    println!("========================================================");
    println!("  NYXI Striker KEYBOARD: OPCODE 4 vs 7 'HELLO WORLD' TEST   ");
    println!("========================================================");

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize hidapi: {}", e);
            return;
        }
    };

    let mut iface2 = None;
    for dev_info in api.device_list() {
        if dev_info.vendor_id() == 0x3151 && dev_info.product_id() == 0x4015 {
            if dev_info.interface_number() == 2 && dev_info.usage() == 2 && dev_info.usage_page() == 0xffff {
                iface2 = Some(dev_info.clone());
            }
        }
    }

    let dev_info = match iface2 {
        Some(d) => d,
        None => {
            eprintln!("Interface 2 not found!");
            return;
        }
    };

    println!("Opening Interface 2 at {:?}", dev_info.path());
    let dev = match dev_info.open_device(&api) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open: {}", e);
            return;
        }
    };

    // Helper: send with report ID 0 (65 bytes)
    let send65 = |dev: &hidapi::HidDevice, msg: &[u8; 64]| {
        let mut rep = [0u8; 65];
        rep[0] = 0x00;
        rep[1..65].copy_from_slice(msg);
        let _ = dev.send_feature_report(&rep);
    };

    // Helper: send raw (64 bytes)
    let send64 = |dev: &hidapi::HidDevice, msg: &[u8; 64]| {
        let _ = dev.send_feature_report(msg);
    };

    // -------------------------------------------------------------
    // Query current settings using 0x84 (GET_LEDPARAM for Vx)
    // -------------------------------------------------------------
    let mut q_84 = [0u8; 64];
    q_84[0] = 0x84; // FEA_CMD_GET_LEDPARAM (Vx = 132 = 0x84)
    send65(&dev, &q_84);
    sleep(Duration::from_millis(50));
    let mut resp = [0u8; 65];
    if let Ok(n) = dev.get_feature_report(&mut resp) {
        println!("Response to 0x84 query ({} bytes): {:02x?}", n, &resp[..n.min(16)]);
    }

    // -------------------------------------------------------------
    // Query current settings using 0x87 (GET_LEDPARAM for Kx)
    // -------------------------------------------------------------
    let mut q_87 = [0u8; 64];
    q_87[0] = 0x87; // FEA_CMD_GET_LEDPARAM (Kx = 135 = 0x87)
    send65(&dev, &q_87);
    sleep(Duration::from_millis(50));
    if let Ok(n) = dev.get_feature_report(&mut resp) {
        println!("Response to 0x87 query ({} bytes): {:02x?}", n, &resp[..n.min(16)]);
    }

    // =============================================================
    // TEST PHASE 1: Opcode 4 (Vx Protocol)
    // =============================================================
    println!("\n========================================================");
    println!("PHASE 1: Testing Opcode 0x04 (Vx Protocol)");
    println!("========================================================");

    // Command 1: Opcode 4, Mode 0 (Light OFF)
    let mut off_4 = [0u8; 64];
    off_4[0] = 0x04;
    off_4[1] = 0x00; // LightOff
    send65(&dev, &off_4);
    countdown(4, "[TEST 1] Sent Opcode 0x04: Turn Backlight OFF");

    // Command 2: Opcode 4, Mode 1 (Always ON, max brightness = 4)
    let mut on_4 = [0u8; 64];
    on_4[0] = 0x04;
    on_4[1] = 0x01; // AlwaysOn
    on_4[2] = 0x01; // 5 - speed(4) = 1
    on_4[3] = 0x04; // brightness = 4 (max)
    on_4[4] = 0x00;
    send65(&dev, &on_4);
    countdown(4, "[TEST 2] Sent Opcode 0x04: Turn Backlight ON (Mode 1, Brightness Max)");

    // Command 3: Opcode 4, Mode 3 (Wave)
    let mut wave_4 = [0u8; 64];
    wave_4[0] = 0x04;
    wave_4[1] = 0x03; // LightWave
    wave_4[2] = 0x01; // Speed
    wave_4[3] = 0x04; // Brightness
    wave_4[4] = 0x00; // Right
    send65(&dev, &wave_4);
    countdown(4, "[TEST 3] Sent Opcode 0x04: LightWave (Mode 3)");

    // Repeat Opcode 4 raw (64 bytes)
    send64(&dev, &off_4);
    countdown(4, "[TEST 4] Sent Opcode 0x04 (RAW 64 bytes): Turn Backlight OFF");

    send64(&dev, &on_4);
    countdown(4, "[TEST 5] Sent Opcode 0x04 (RAW 64 bytes): Turn Backlight ON");

    // =============================================================
    // TEST PHASE 2: Opcode 7 (Kx Protocol)
    // =============================================================
    println!("\n========================================================");
    println!("PHASE 2: Testing Opcode 0x07 (Kx Protocol with RGB)");
    println!("========================================================");

    let mut off_7 = [0u8; 64];
    off_7[0] = 0x07;
    off_7[1] = 0x00;
    send65(&dev, &off_7);
    countdown(4, "[TEST 6] Sent Opcode 0x07: Turn Backlight OFF");

    let mut red_7 = [0u8; 64];
    red_7[0] = 0x07;
    red_7[1] = 0x01; // AlwaysOn
    red_7[2] = 0x04; // Speed
    red_7[3] = 0x04; // Brightness
    red_7[4] = 0x07; // NORMAL (single color)
    red_7[5] = 0xFF; // R = 255
    send65(&dev, &red_7);
    countdown(4, "[TEST 7] Sent Opcode 0x07: Solid RED");

    println!("\n========================================================");
    println!("Test complete!");
    println!("========================================================");
}
