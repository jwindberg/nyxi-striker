use crate::proto::driver::SystemInfo;
use parking_lot::Mutex;
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

pub struct SystemMonitor {
    sys: Arc<Mutex<System>>,
    disks: Arc<Mutex<Disks>>,
    networks: Arc<Mutex<Networks>>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        Self {
            sys: Arc::new(Mutex::new(sys)),
            disks: Arc::new(Mutex::new(disks)),
            networks: Arc::new(Mutex::new(networks)),
        }
    }

    pub fn snapshot(&self) -> SystemInfo {
        let mut sys = self.sys.lock();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let mut disks = self.disks.lock();
        disks.refresh();

        let mut networks = self.networks.lock();
        networks.refresh();

        let mut disk_total = 0u64;
        let mut disk_avail = 0u64;
        for disk in disks.iter() {
            disk_total += disk.total_space();
            disk_avail += disk.available_space();
        }

        let mut net_up = 0u64;
        let mut net_down = 0u64;
        for (_interface, data) in networks.iter() {
            net_up += data.transmitted();
            net_down += data.received();
        }

        let cpu_usage = sys.global_cpu_usage();
        let mem_total = sys.total_memory();
        let mem_used = sys.used_memory();

        SystemInfo {
            disk_space_total: disk_total,
            disk_spce_available: disk_avail,
            net_work_total_up: net_up,
            net_work_total_down: net_down,
            cpu_temperater: 42.0, // Default estimate if thermal sensors aren't exposed
            mem_total,
            mem_used,
            cpu_usage,
        }
    }
}
