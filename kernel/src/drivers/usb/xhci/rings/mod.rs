pub mod command;
pub mod event;
pub mod transfer;

use crate::{
    drivers::usb::xhci::trb::Trb,
    memory::physical::{PhysRegion, allocate_frame},
};
use core::{marker::PhantomData, ops::Add};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{PageSize, Size4KiB},
};

const TRB_COUNT: usize = Size4KiB::SIZE as usize / size_of::<Trb>(); // 256 TRBs per ring

#[derive(Debug)]
pub struct Ring<K> {
    pub phys_region: PhysRegion,
    pub next: *mut Trb,
    pub cycle: bool,
    pub link_trb: *mut Trb,
    _kind: PhantomData<K>,
}

impl<K> Ring<K> {
    pub fn new(initial_cycle: bool, with_link_trb: bool) -> Self {
        let phys_region = allocate_frame();

        let link_trb_ptr = unsafe { phys_region.virt.as_mut_ptr::<Trb>().add(TRB_COUNT - 1) };
        if with_link_trb {
            unsafe { link_trb_ptr.write_volatile(Trb::new_link(phys_region.phys, initial_cycle)) };
        }

        Self {
            phys_region,
            next: phys_region.virt.as_mut_ptr::<Trb>(),
            cycle: initial_cycle,
            link_trb: link_trb_ptr,
            _kind: PhantomData,
        }
    }
    pub fn get_next_phys(&self) -> PhysAddr {
        let offset = VirtAddr::from_ptr(self.next) - self.phys_region.virt;
        self.phys_region.phys.add(offset)
    }
}
