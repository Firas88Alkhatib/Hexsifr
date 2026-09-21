use x86_64::PhysAddr;

// DWORD 0
const EP_STATE_MASK: u32 = 0x0000_0007;
const EP_STATE_SHIFT: u32 = 0;
const MULT_MASK: u32 = 0x0000_0018; // bits 4:3 (2 bits)
const MULT_SHIFT: u32 = 3;
const INTERVAL_MASK: u32 = 0x00FF_0000; // bits 23:16
const INTERVAL_SHIFT: u32 = 16;

// DWORD 1
const CERR_MASK: u32 = 0x0000_0006; // bits 2:1
const CERR_SHIFT: u32 = 1;
const EP_TYPE_MASK: u32 = 0x0000_0038; // bits 5:3
const EP_TYPE_SHIFT: u32 = 3;
const HID_MASK: u32 = 0x0000_0080; // bit 7
const BURST_SIZE_MASK: u32 = 0x0000_FF00; // bits 15:8
const BURST_SIZE_SHIFT: u32 = 8;
const MAX_PACKET_SIZE_MASK: u32 = 0xFFFF_0000; // bits 31:16
const MAX_PACKET_SIZE_SHIFT: u32 = 16;

// DWORD 2/3
const TR_DEQUEUE_PTR_MASK: u64 = !0x0F; // clear low 4 bits
const DCS_BIT: u32 = 1 << 0; // Dequeue Cycle State

// DWORD 4
const AVG_TRB_LENGTH_MASK: u32 = 0x0000_FFFF;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EndpointContext {
    pub dword0: u32,
    pub dword1: u32,
    pub tr_dequeue_ptr_lo: u32,
    pub tr_dequeue_ptr_hi: u32,
    pub dword4: u32,
    pub dword5: u32,
    pub reserved: [u32; 2],
}

impl EndpointContext {
    pub fn for_control_ep0(tansfer_ring_phys: PhysAddr, max_packet_size: u16) -> Self {
        let phys = tansfer_ring_phys.as_u64();

        Self {
            // MaxBurst = 0, EPType = 4 (Control)
            // dword1: (max_packet_size as u32) << 16 | (4u32 << 3),
            dword1: (3 << 1) | (4 << 3) | ((max_packet_size as u32) << 16),
            tr_dequeue_ptr_lo: (phys as u32 & !0xF) | 1,
            tr_dequeue_ptr_hi: (phys >> 32) as u32,
            dword4: 8,
            ..Default::default()
        }
    }
    pub fn for_interrupt_in(transfer_ring_phys: PhysAddr, max_packet_size: u16, interval: u8) -> Self {
        let phys = transfer_ring_phys.as_u64();
        Self {
            dword0: (interval as u32) << 16,
            dword1: (3 << 1)                        // CErr = 3
                | (7 << 3)                          // EP Type = Interrupt IN
                | ((max_packet_size as u32) << 16),
            tr_dequeue_ptr_lo: (phys as u32 & !0xF) | 1,
            tr_dequeue_ptr_hi: (phys >> 32) as u32,
            dword4: max_packet_size as u32,
            ..Default::default()
        }
    }
}
