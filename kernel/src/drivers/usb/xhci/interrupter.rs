use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::drivers::usb::xhci::{
    mmio::{Mmio, Reg},
    rings::{
        Ring,
        event::{Erst, EventRing},
    },
    trb::event::Event,
};
#[repr(C)]
pub struct InterrupterRegs {
    pub iman: Reg<u32>,
    pub imod: Reg<u32>,
    pub erstsz: Reg<u32>,
    _reserved: Reg<u32>,
    pub erstba: Reg<u64>,
    pub erdp: Reg<u64>,
}

pub struct Interrupter {
    mmio: Mmio<InterrupterRegs>,
    event_ring: Ring<EventRing>,
    erst: Erst,
}

impl Interrupter {
    pub fn new(addr: VirtAddr) -> Self {
        let event_ring = Ring::<EventRing>::new(true, false);
        let erst = Erst::new(event_ring.phys_region.phys);
        Self { mmio: Mmio::<InterrupterRegs>::new(addr), event_ring, erst }
    }

    pub fn init(&self) {
        self.set_event_ring_segment_table_size(1);
        self.set_event_ring_segment_table_base();
        self.update_erdp();
        self.enable();
    }

    pub fn update_erdp(&self) {
        const ERDP_EHB: u64 = 1 << 3;
        self.mmio.erdp.write(self.event_ring.get_next_phys().as_u64() | ERDP_EHB);
    }

    pub fn enable(&self) {
        const IMAN_INTERRUPT_ENABLE: u32 = 1 << 1; // bit 1
        self.mmio.iman.modify(|v| v | IMAN_INTERRUPT_ENABLE);
    }
    pub fn set_event_ring_segment_table_size(&self, size: u32) {
        self.mmio.erstsz.write(size);
    }

    pub fn set_event_ring_segment_table_base(&self) {
        self.mmio.erstba.write(self.erst.phys_region.phys.as_u64());
    }
    /// Clears pending state
    pub fn eoi(&self) {
        const IMAN_INTERRUPT_PENDING: u32 = 1 << 0;
        self.mmio.iman.modify(|v| v | IMAN_INTERRUPT_PENDING);
    }
    pub fn dequeue_events(&mut self) -> Vec<Event> {
        let events = core::iter::from_fn(|| self.event_ring.dequeue_next_event()).collect();
        self.update_erdp();
        events
    }
}
