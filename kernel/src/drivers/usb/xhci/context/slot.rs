use crate::drivers::usb::xhci::ports::UsbSpeed;

const ROUTE_STRING_MASK: u32 = 0x000F_FFFF; // bits 19:0
const SPEED_MASK: u32 = 0x00F0_0000; // bits 23:20
const CONTEXT_ENTRIES_MASK: u32 = 0xF800_0000; // bits 31:27

const ROUTE_STRING_SHIFT: u32 = 0;
const SPEED_SHIFT: u32 = 20;
const CONTEXT_ENTRIES_SHIFT: u32 = 27;

// DWORD 1
const RH_PORT_MASK: u32 = 0x00FF_0000; // bits 23:16
const RH_PORT_SHIFT: u32 = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SlotContext {
    pub dword0: u32,
    // pub dword1: u32,
    pub max_exit_latency_lo: u8, // offset 4
    pub max_exit_latency_hi: u8, // offset 5
    pub root_hub_port_number: u8,
    pub number_of_ports: u8,
    pub dword2: u32,
    pub dword3: u32,
    pub dword4: u32,
    pub dword5: u32,
    pub dword6: u32,
    pub dword7: u32,
}

impl SlotContext {
    pub fn for_address_device(speed: UsbSpeed, port_id: u8) -> Self {
        Self { dword0: (1 << 27) | (u32::from(speed) << 20), root_hub_port_number: port_id, ..Default::default() }
    }
}
