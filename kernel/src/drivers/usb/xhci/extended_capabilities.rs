use core::ops::Add;

use alloc::collections::BTreeMap;
use x86_64::VirtAddr;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtCapId {
    UsbLegacySupport = 1,
    SupportedProtocol = 2,
    PowerManagement = 3,
    Virtualization = 4,
    RouteString = 5,

    Debug = 10,

    Other(u8),
}

impl From<u8> for ExtCapId {
    fn from(value: u8) -> Self {
        match value {
            1 => ExtCapId::UsbLegacySupport,
            2 => ExtCapId::SupportedProtocol,
            3 => ExtCapId::PowerManagement,
            4 => ExtCapId::Virtualization,
            5 => ExtCapId::RouteString,
            6 => ExtCapId::Debug,
            _ => ExtCapId::Other(value),
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtCapHeader {
    pub id: u8,
    pub next: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UsbRevision {
    pub revision_minor: u8,
    pub revision_major: u8,
}
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SupportedProtocol {
    pub header: ExtCapHeader,
    pub revision: UsbRevision,
    pub name_string: u32,
    pub port_offset: u8,
    pub port_count: u8,
    pub protocol_info: u16,
    pub slot_type: u8,
}

impl SupportedProtocol {
    pub fn protocol_defined(&self) -> u16 {
        self.protocol_info & 0x0fff
    }

    pub fn psic(&self) -> u8 {
        (self.protocol_info >> 12) as u8
    }
}

pub fn get_ports_versions_protocols(bar_virt: VirtAddr, xecp_offset: u32) -> BTreeMap<u8, UsbRevision> {
    let mut ports_map: BTreeMap<u8, UsbRevision> = BTreeMap::new();

    if xecp_offset == 0 {
        return ports_map;
    }

    let mut next = bar_virt.add(xecp_offset as u64);
    loop {
        let header = unsafe { next.as_ptr::<ExtCapHeader>().read_volatile() };
        match ExtCapId::from(header.id) {
            ExtCapId::UsbLegacySupport => {}
            ExtCapId::SupportedProtocol => {
                let protocol = unsafe { next.as_ptr::<SupportedProtocol>().read_volatile() };
                let start = protocol.port_offset;
                let end = start + protocol.port_count;
                for i in start..end {
                    ports_map.insert(i, protocol.revision);
                }
            }
            ExtCapId::PowerManagement => {}
            ExtCapId::Virtualization => {}
            ExtCapId::RouteString => {}
            ExtCapId::Debug => {}
            ExtCapId::Other(_) => {}
        }

        if header.next == 0 {
            break;
        }
        next = next.add(header.next as u64 * 4);
    }
    ports_map
}
