use crate::config::SupportConfig;
use crate::proto::driver::*;
use hidapi::{HidApi, HidDevice};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct DeviceManager {
    config: SupportConfig,
    hid_api: Arc<Mutex<HidApi>>,
    devices: Arc<Mutex<HashMap<String, Arc<Mutex<HidDevice>>>>>,
    broadcaster: broadcast::Sender<DeviceList>,
    last_device_list: Arc<Mutex<Vec<DjDev>>>,
}

impl DeviceManager {
    pub fn new(config: SupportConfig) -> Result<Self, String> {
        let hid_api = HidApi::new().map_err(|e| format!("Failed to init HID API: {}", e))?;
        let (broadcaster, _) = broadcast::channel(32);

        Ok(Self {
            config,
            hid_api: Arc::new(Mutex::new(hid_api)),
            devices: Arc::new(Mutex::new(HashMap::new())),
            broadcaster,
            last_device_list: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DeviceList> {
        self.broadcaster.subscribe()
    }

    pub fn current_device_list(&self) -> DeviceList {
        let mut list = self.last_device_list.lock().clone();
        if list.is_empty() {
            let scanned = self.scan_devices();
            *self.last_device_list.lock() = scanned.clone();
            list = scanned;
        }
        DeviceList {
            devlist: list,
            r#type: DeviceListChangeType::Init as i32,
        }
    }

    pub fn scan_devices(&self) -> Vec<DjDev> {
        let mut api = self.hid_api.lock();
        let _ = api.refresh_devices();

        let mut result = Vec::new();

        for dev_info in api.device_list() {
            let vid = dev_info.vendor_id();
            let pid = dev_info.product_id();
            let usage = dev_info.usage();
            let usage_page = dev_info.usage_page();
            let interface = dev_info.interface_number();

            if let Some(matched) = self.config.is_supported(vid, pid, usage, usage_page, interface) {
                let path_str = dev_info.path().to_string_lossy().to_string();
                let dev_id = match (vid, pid) {
                    (0x3151, 0x4015) => 1473,
                    (0x3151, 0x4011) => 2147485121u32,
                    (0x3151, 0x4038) => 1473,
                    _ => ((vid as u32) << 16) | (pid as u32),
                };

                let is_boot = self.config.boot.iter().any(|b| {
                    b.vid.map(|v| v as u16 == vid).unwrap_or(false)
                        && b.pid.map(|p| p as u16 == pid).unwrap_or(false)
                });
                let is_vender = self.config.vender.iter().any(|v| {
                    v.vid.map(|v_id| v_id as u16 == vid).unwrap_or(false)
                        && v.pid.map(|p| p as u16 == pid).unwrap_or(false)
                });

                let devtype = if is_boot {
                    DeviceType::Yzwboot as i32
                } else if is_vender {
                    DeviceType::Yzwvender as i32
                } else {
                    DeviceType::Yzwkeyboard as i32
                };

                if matched.dongle_common {
                    let dangle = DangleCommon {
                        keyboard: Some(DangleStatus {
                            status_oneof: Some(dangle_status::StatusOneof::Status(Status24 {
                                battery: 100,
                                is_online: true,
                            })),
                        }),
                        mouse: Some(DangleStatus {
                            status_oneof: Some(dangle_status::StatusOneof::Empty(Empty {})),
                        }),
                        path: path_str,
                        keyboard_id: dev_id,
                        mouse_id: 0,
                        vid: vid as u32,
                        pid: pid as u32,
                    };
                    result.push(DjDev {
                        dev_oneof: Some(dj_dev::DevOneof::Danglecommondev(dangle)),
                    });
                } else {
                    let dev = Device {
                        devtype,
                        is24: matched.dongle_common,
                        path: path_str,
                        id: dev_id,
                        battery: 100,
                        isonline: true,
                        vid: vid as u32,
                        pid: pid as u32,
                    };
                    result.push(DjDev {
                        dev_oneof: Some(dj_dev::DevOneof::Dev(dev)),
                    });
                }
            }
        }

        result
    }

    pub fn start_monitor_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
            loop {
                interval.tick().await;
                let current = self.scan_devices();
                let mut last = self.last_device_list.lock();

                let changed = current.len() != last.len() || !current.iter().zip(last.iter()).all(|(a, b)| {
                    match (&a.dev_oneof, &b.dev_oneof) {
                        (Some(dj_dev::DevOneof::Dev(d1)), Some(dj_dev::DevOneof::Dev(d2))) => d1.path == d2.path,
                        (Some(dj_dev::DevOneof::Danglecommondev(d1)), Some(dj_dev::DevOneof::Danglecommondev(d2))) => d1.path == d2.path,
                        _ => false,
                    }
                });

                if changed {
                    tracing::info!("Detected device list change ({} devices active)", current.len());
                    *last = current.clone();
                    let msg = DeviceList {
                        devlist: current,
                        r#type: DeviceListChangeType::Change as i32,
                    };
                    let _ = self.broadcaster.send(msg);
                }
            }
        });
    }

    fn get_device(&self, path: &str) -> Result<Arc<Mutex<HidDevice>>, String> {
        let mut devs = self.devices.lock();
        if let Some(dev) = devs.get(path) {
            return Ok(dev.clone());
        }

        let api = self.hid_api.lock();
        let c_path = CString::new(path).map_err(|e| format!("Invalid path {}: {}", path, e))?;
        let dev = api.open_path(&c_path).map_err(|e| format!("Cannot open HID device {}: {}", path, e))?;

        let arc_dev = Arc::new(Mutex::new(dev));
        devs.insert(path.to_string(), arc_dev.clone());
        Ok(arc_dev)
    }

    pub fn send_msg(&self, device_path: &str, msg: Vec<u8>, checksum_type: CheckSumType, _dangle_type: DangleDevType) -> Result<(), String> {
        if device_path.is_empty() {
            return Err("Empty device path".to_string());
        }

        // Feature reports are always 64 bytes; the app may send shorter buffers.
        let mut msg = msg;
        msg.resize(64, 0);

        // BIT7/BIT8: checksum byte at index 7/8 = 0xFF - sum of the bytes before it.
        let checksum_pos = match checksum_type {
            CheckSumType::Bit7 => Some(7),
            CheckSumType::Bit8 => Some(8),
            CheckSumType::None => None,
        };
        if let Some(pos) = checksum_pos {
            let sum = msg[..pos].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
            msg[pos] = 0xFF - sum;
        }

        let dev = self.get_device(device_path)?;
        let locked_dev = dev.lock();

        // Report ID 0 prefix + 64-byte payload
        let mut report = vec![0u8; 65];
        report[1..].copy_from_slice(&msg);
        locked_dev
            .send_feature_report(&report)
            .map_err(|e| format!("HID send failed: {}", e))?;

        Ok(())
    }

    pub fn read_msg(&self, device_path: &str) -> Result<Vec<u8>, String> {
        if device_path.is_empty() {
            return Err("Empty device path".to_string());
        }

        let dev = self.get_device(device_path)?;
        let locked_dev = dev.lock();

        // buf[0] is the report ID (0); the payload follows.
        let mut buf = vec![0u8; 65];
        let n = locked_dev
            .get_feature_report(&mut buf)
            .map_err(|e| format!("HID read error: {}", e))?;
        Ok(buf[1..n.max(1)].to_vec())
    }

    pub fn clean_dev(&self, device_path: &str) {
        let mut devs = self.devices.lock();
        devs.remove(device_path);
    }
}
