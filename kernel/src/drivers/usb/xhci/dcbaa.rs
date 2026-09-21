use x86_64::PhysAddr;

use crate::memory::physical::{PhysRegion, allocate_frame};

pub struct Dcbaa {
    pub phys_region: PhysRegion,
}

impl Dcbaa {
    pub fn new(num_buffers: u32) -> Self {
        let phys_region = allocate_frame();
        if num_buffers > 0 {
            let scratchpad_buffers = ScratchpadArray::new(num_buffers);
            phys_region.write_as(scratchpad_buffers.phys_region.phys.as_u64());
        }
        Self { phys_region }
    }

    pub fn set(&mut self, slot_id: u8, device_context_phys: PhysAddr) {
        assert!(slot_id > 0, "Slot ID must be greater than zero since slot 0 is reserved for the Scratchpad array.");

        unsafe {
            self.phys_region.virt.as_mut_ptr::<u64>().add(slot_id as usize).write_volatile(device_context_phys.as_u64())
        }
    }
}

pub struct ScratchpadArray {
    phys_region: PhysRegion,
}
impl ScratchpadArray {
    pub fn new(num_buffers: u32) -> Self {
        let phys_region = allocate_frame();

        for i in 0..num_buffers {
            let buffer_phys_region = allocate_frame();
            unsafe {
                phys_region.virt.as_mut_ptr::<u64>().add(i as usize).write_volatile(buffer_phys_region.phys.as_u64())
            }
        }

        Self { phys_region }
    }
}
