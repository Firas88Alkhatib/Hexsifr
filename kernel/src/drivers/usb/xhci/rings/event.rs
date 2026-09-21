use x86_64::PhysAddr;

use crate::{
    drivers::usb::xhci::{
        rings::{Ring, TRB_COUNT},
        trb::{Trb, event::Event},
    },
    memory::physical::{PhysRegion, allocate_frame},
};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ErstEntry {
    pub ring_segment_base: u64,
    pub ring_segment_size: u16,
    pub reserved: u16,
    pub reserved2: u32,
}

pub struct Erst {
    pub phys_region: PhysRegion,
}
impl Erst {
    pub fn new(event_ring_phys: PhysAddr) -> Self {
        let phys_region = allocate_frame();
        let entry = ErstEntry {
            ring_segment_base: event_ring_phys.as_u64(),
            ring_segment_size: TRB_COUNT as u16,
            reserved: 0,
            reserved2: 0,
        };

        unsafe { phys_region.virt.as_mut_ptr::<ErstEntry>().write(entry) }
        Self { phys_region }
    }
}

pub struct EventRing;

impl Ring<EventRing> {
    fn advance(&mut self) {
        self.next = unsafe { self.next.add(1) };

        if self.next == unsafe { self.link_trb.add(1) } {
            self.next = self.phys_region.virt.as_mut_ptr::<Trb>();
            self.cycle = !self.cycle;
        }
    }
    pub fn dequeue_next_event(&mut self) -> Option<Event> {
        let next_trb = unsafe { self.next.read() };
        if next_trb.get_cycle() != self.cycle {
            return None;
        }
        self.advance();
        return Some(next_trb.into());
    }
}
