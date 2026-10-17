use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("=======================================================");
    println!("       NYXI CA01: SINGLE VERIFIED SOLID RED LED TEST   ");
    println!("=======================================================");

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize HIDAPI: {}", e);
            return;
        }
    };

    let mut target_device = None;

    for dev_info in api.device_list() {
        let vid = dev_info.vendor_id();
        let pid = dev_info.product_id();
        let iface = dev_info.interface_number();
        let up = dev_info.usage_page();
        let u = dev_info.usage();

        if vid == 0x3151 && (pid == 0x4015 || pid == 0x4011) {
            let mode = if pid == 0x4015 { "WIRED (0x4015)" } else { "DONGLE (0x4011)" };
            println!("Found candidate: {} [iface={}, up=0x{:04x}, u=0x{:04x}]", mode, iface, up, u);

            // Interface 2 (usage_page 0xffff, usage 2) is the vendor control interface
            if iface == 2 || (up == 0xffff && u == 2) {
                match dev_info.open_device(&api) {
                    Ok(dev) => {
                        println!("==> Successfully opened Vendor Control Interface: {} [iface={}]", mode, iface);
                        target_device = Some((format!("{} iface={}", mode, iface), dev));
                        break;
                    }
                    Err(e) => {
                        eprintln!("\n[!] Failed to open vendor interface: {}", e);
                        if e.to_string().contains("0xE00002C5") || e.to_string().contains("exclusive access") {
                            eprintln!("    CAUSE: Another process (such as nyxi-driver daemon or iot_driver) currently holds exclusive access.");
                            eprintln!("    FIX: Run `killall nyxi-driver iot_driver` in terminal before testing.");
                        }
                        return;
                    }
                }
            }
        }
    }

    let (name, dev) = match target_device {
        Some(d) => d,
        None => {
            println!("\n[!] Vendor Control Interface (iface 2) not found.");
            println!("    Please verify that the NYXI keyboard is connected via USB cable or 2.4G dongle.");
            return;
        }
    };

    println!("\n>>> Target: [{}]", name);

    // =========================================================================
    // STEP 1: Query Keyboard Options (0x86) to check power-saving / link health
    // =========================================================================
    println!("\n[STEP 1] Querying Keyboard Options (FEA_CMD_GET_KBOPTION = 0x86)...");
    let mut q_kbopt = [0u8; 65];
    q_kbopt[0] = 0x00;
    q_kbopt[1] = 0x86;
    // Real driver uses BIT7 checksum (& 0x7F) for GET_KBOPTION
    let sum_opt: u32 = q_kbopt[1..64].iter().map(|&b| b as u32).sum();
    q_kbopt[64] = (sum_opt & 0x7F) as u8; // 0x06

    if let Err(e) = dev.send_feature_report(&q_kbopt) {
        eprintln!("[FAIL at STEP 1] Failed to send GET_KBOPTION query: {}", e);
        return;
    }
    sleep(Duration::from_millis(50));

    let mut resp_opt = [0u8; 65];
    match dev.get_feature_report(&mut resp_opt) {
        Ok(n) => {
            println!("  -> Received {} bytes response from MCU", n);
            let byte2 = resp_opt[2];
            let led_off = (byte2 & 16) != 0;
            println!("  -> Raw flags (byte 2): 0x{:02x} (LED Off / PowerSave: {})", byte2, led_off);
            if led_off {
                eprintln!("[WARN] The keyboard MCU has reported LED_OFF / PowerSave is ACTIVE.");
                eprintln!("       The hardware will reject lighting changes until power save is disabled.");
            }
        }
        Err(e) => {
            println!("  -> Note: GET_KBOPTION readback returned: {} (continuing)", e);
        }
    }

    // =========================================================================
    // STEP 2: Query Initial LED State (0x87)
    // =========================================================================
    println!("\n[STEP 2] Querying Initial Hardware LED State (FEA_CMD_GET_LEDPARAM = 0x87)...");
    let mut q_led = [0u8; 65];
    q_led[0] = 0x00;
    q_led[1] = 0x87;
    // Real driver uses BIT7 checksum (& 0x7F) for GET_LEDPARAM
    let sum_led: u32 = q_led[1..64].iter().map(|&b| b as u32).sum();
    q_led[64] = (sum_led & 0x7F) as u8; // 0x07

    if let Err(e) = dev.send_feature_report(&q_led) {
        eprintln!("[FAIL at STEP 2] Failed to send GET_LEDPARAM query: {}", e);
        return;
    }
    sleep(Duration::from_millis(50));

    let mut resp_initial = [0u8; 65];
    match dev.get_feature_report(&mut resp_initial) {
        Ok(_) => {
            let mode = resp_initial[1];
            let speed = 4 - resp_initial[2];
            let brightness = resp_initial[3];
            let opt = resp_initial[4];
            let (r, g, b) = (resp_initial[5], resp_initial[6], resp_initial[7]);
            println!("  -> Initial Hardware State: Mode={}, Speed={}, Brightness={}, Opt=0x{:02x}, RGB=({}, {}, {})",
                mode, speed, brightness, opt, r, g, b);
        }
        Err(e) => {
            println!("  -> Note: Initial GET_LEDPARAM readback returned: {} (continuing)", e);
        }
    }

    // =========================================================================
    // STEP 3: Dispatch Solid RED Feature Report (0x07)
    // =========================================================================
    println!("\n[STEP 3] Dispatching Solid RED Feature Report (FEA_CMD_SET_LEDPARAM = 0x07)...");
    let mut buf = [0u8; 65];
    buf[0] = 0x00; // Report ID 0 prefix
    buf[1] = 0x07; // FEA_CMD_SET_LEDPARAM
    buf[2] = 0x01; // Mode 1: LightAlwaysOn
    buf[3] = 0x04; // Speed: 4 (MAXSPEED - 0 = 4)
    buf[4] = 0x04; // Brightness: 4 (maximum)
    buf[5] = 0x07; // Option: 7 (a.NORMAL = 7 for Custom RGB mode)
    buf[6] = 0xFF; // R: 255 (Red)
    buf[7] = 0x00; // G: 0
    buf[8] = 0x00; // B: 0

    // Real driver uses BIT8 checksum (& 0xFF) for SET_LEDPARAM
    let sum: u32 = buf[1..64].iter().map(|&b| b as u32).sum();
    let checksum = (sum & 0xFF) as u8; // 0x15
    buf[64] = checksum;

    println!("  -> Payload: cmd=0x{:02x}, mode=0x{:02x}, speed=0x{:02x}, brt=0x{:02x}, opt=0x{:02x}, RGB=({}, {}, {}), chk=0x{:02x}",
        buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8], checksum);

    match dev.send_feature_report(&buf) {
        Ok(_) => {
            println!("  -> [OK] USB Control Transfer completed successfully.");
        }
        Err(e) => {
            eprintln!("[FAIL at STEP 3] send_feature_report returned error: {}", e);
            return;
        }
    }

    // Allow hardware MCU 100ms to commit to internal registers
    sleep(Duration::from_millis(150));

    // =========================================================================
    // STEP 4: Query Post-Command Hardware State (0x87 Verification Readback)
    // =========================================================================
    println!("\n[STEP 4] Performing Verification Readback (FEA_CMD_GET_LEDPARAM = 0x87)...");
    let mut verify_q = [0u8; 65];
    verify_q[0] = 0x00;
    verify_q[1] = 0x87;
    let v_sum: u32 = verify_q[1..64].iter().map(|&b| b as u32).sum();
    verify_q[64] = (v_sum & 0x7F) as u8; // 0x07

    if let Err(e) = dev.send_feature_report(&verify_q) {
        eprintln!("[FAIL at STEP 4] Failed to send verification query: {}", e);
        return;
    }
    sleep(Duration::from_millis(50));

    let mut resp_after = [0u8; 65];
    match dev.get_feature_report(&mut resp_after) {
        Ok(_) => {
            let mode = resp_after[1];
            let speed = resp_after[2];
            let brightness = resp_after[3];
            let opt = resp_after[4];
            let (r, g, b) = (resp_after[5], resp_after[6], resp_after[7]);

            println!("  -> Hardware Register Readback: Mode={}, Speed={}, Brightness={}, Opt={}, RGB=({}, {}, {})",
                mode, speed, brightness, opt, r, g, b);

            println!("\n=======================================================");
            if mode == 0x01 && r == 0xFF && g == 0x00 && b == 0x00 {
                println!(" [VERIFIED SUCCESS] Keyboard MCU confirmed state transition!");
                println!(" Hardware state matches command: Mode 1 (AlwaysOn), Solid RED (255, 0, 0).");
            } else {
                println!(" [STATE MISMATCH] MCU returned Mode={}, RGB=({}, {}, {}) instead of Solid RED.", mode, r, g, b);
                println!(" The MCU rejected the parameters or did not transition.");
            }
            println!("=======================================================");
        }
        Err(e) => {
            eprintln!("[FAIL at STEP 4] Verification get_feature_report failed: {}", e);
        }
    }

    println!("\nHolding connection for 2 seconds...");
    sleep(Duration::from_secs(2));
    println!("Test finished.");
}
