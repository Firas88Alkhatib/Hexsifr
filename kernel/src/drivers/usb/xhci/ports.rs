use core::hint::spin_loop;

use alloc::{collections::BTreeMap, vec::Vec};
use x86_64::VirtAddr;

use crate::drivers::usb::xhci::{
    extended_capabilities::UsbRevision,
    mmio::{Mmio, Reg},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    Invalid,
    Full,
    Low,
    High,
    Super,
    SuperPlus,
    Other(u8),
}
impl UsbSpeed {
    pub fn max_packet_size(&self) -> u16 {
        match self {
            UsbSpeed::Low | UsbSpeed::Full => 8,
            UsbSpeed::High => 64,
            UsbSpeed::Super | UsbSpeed::SuperPlus => 512,
            _ => 8,
        }
    }
    pub fn get_interval(&self, descriptor_interval: u8) -> u8 {
        match self {
            UsbSpeed::Super | UsbSpeed::SuperPlus => descriptor_interval,
            _ => descriptor_interval.saturating_sub(1),
        }
    }
    pub fn get_max_packet_size_for_ep0(&self, max_packet_size0: u8) -> u16 {
        match self {
            UsbSpeed::Super | UsbSpeed::SuperPlus => 1u16 << max_packet_size0, // 2^9 = 512
            _ => max_packet_size0 as u16,
        }
    }
}

impl From<u32> for UsbSpeed {
    fn from(value: u32) -> Self {
        match value {
            0 => UsbSpeed::Invalid,
            1 => UsbSpeed::Full,
            2 => UsbSpeed::Low,
            3 => UsbSpeed::High,
            4 => UsbSpeed::Super,
            5 => UsbSpeed::SuperPlus,
            _ => UsbSpeed::Other(value as u8),
        }
    }
}

impl From<UsbSpeed> for u32 {
    fn from(speed: UsbSpeed) -> Self {
        match speed {
            UsbSpeed::Invalid => 0,
            UsbSpeed::Full => 1,
            UsbSpeed::Low => 2,
            UsbSpeed::High => 3,
            UsbSpeed::Super => 4,
            UsbSpeed::SuperPlus => 5,
            UsbSpeed::Other(value) => value as u32,
        }
    }
}

const CCS: u32 = 1 << 0; // Current Connect Status
const PED: u32 = 1 << 1; // Port Enabled/Disabled
const PR: u32 = 1 << 4; // Port Reset
const PP: u32 = 1 << 9; // Port Power

const CSC: u32 = 1 << 17; // Connect Status Change
const PEC: u32 = 1 << 18; // Port Enabled/Disabled Change
const WRC: u32 = 1 << 19; // Warm Port Reset Change
const PRC: u32 = 1 << 21; // Port Reset Change
const WPR: u32 = 1 << 31; // Warm Port Reset

#[repr(C)]
#[derive(Clone)]
pub struct PortRegs {
    pub portsc: Reg<u32>,
    pub portpmsc: Reg<u32>,
    pub portli: Reg<u32>,
    pub porthlpmc: Reg<u32>,
}

const PORT_REG_STRIDE: u64 = 0x10;

pub struct Port {
    mmio: Mmio<PortRegs>,
    pub revision: UsbRevision,
    pub port_id: u8,
    pub addr: VirtAddr,
}
impl Port {
    pub fn new(revision: UsbRevision, addr: VirtAddr, port_id: u8) -> Self {
        Self { mmio: Mmio::<PortRegs>::new(addr), addr, revision, port_id }
    }

    pub fn is_device_connected(&self) -> bool {
        self.mmio.portsc.read() & CCS != 0
    }
    pub fn is_status_changed(&self) -> bool {
        self.mmio.portsc.read() & CSC != 0
    }
    pub fn port_speed(&self) -> UsbSpeed {
        const PS_SHIFT: u32 = 10;
        const PS_MASK: u32 = 0xF << PS_SHIFT;

        let speed = (self.mmio.portsc.read() & PS_MASK) >> PS_SHIFT;
        speed.into()
    }
    pub fn reset(&self) {
        let portsc = &self.mmio.portsc;
        let is_usb3 = self.revision.revision_major == 3;

        // Power on if not
        if portsc.read() & PP == 0 {
            portsc.modify(|v| v | PP);
            while portsc.read() & PP == 0 {
                spin_loop();
            }
        }

        // Clear stale operations
        portsc.modify(|v| v | CSC | PEC | PRC);

        // init reset sequence
        let (reset, change) = if is_usb3 { (WPR, WRC) } else { (PR, PRC) };
        portsc.modify(|v| v | reset);
        // Wait for success
        while portsc.read() & change == 0 {
            core::hint::spin_loop();
        }

        // Acknowledge the reset completion
        portsc.modify(|v| (v | change | CSC | PEC) & !PED);
    }
}

pub struct PortsManager {
    // port_id -> Port
    pub ports: BTreeMap<u8, Port>,
}

impl PortsManager {
    pub fn new(ports_base: VirtAddr, ports_protocols_map: BTreeMap<u8, UsbRevision>) -> Self {
        let mut ports = BTreeMap::new();

        for (port_number, revision) in ports_protocols_map {
            // according to specs, the PORTSC address is:
            // Operational Base + (400h + (10h * (n -1)))
            // where n = Port Number , starting from 1
            // The 400h offset should be added to the op base and provided as ports_base
            let port_offset = PORT_REG_STRIDE * (port_number as u64 - 1);
            let addr = ports_base + port_offset;
            ports.insert(port_number, Port::new(revision, addr, port_number));
        }

        Self { ports }
    }

    pub fn reset_connected_ports(&self) -> Vec<&Port> {
        let mut reset_port_ids: Vec<&Port> = Vec::new();

        for port in self.ports.values() {
            if port.is_device_connected() {
                port.reset();
                reset_port_ids.push(port);
            }
        }
        reset_port_ids
    }
}
