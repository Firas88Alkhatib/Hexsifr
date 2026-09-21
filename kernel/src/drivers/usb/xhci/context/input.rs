use core::ops::Add;

use crate::{
    drivers::usb::xhci::context::{endpoint::EndpointContext, slot::SlotContext},
    memory::physical::{PhysRegion, allocate_frame},
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InputControlContext {
    pub drop_context_flags: u32,
    pub add_context_flags: u32,
    pub reserved: [u32; 6],
}

impl InputControlContext {
    pub fn for_address_device() -> Self {
        Self { add_context_flags: 0b11, ..Default::default() }
    }
}

pub struct InputContext {
    pub phys_region: PhysRegion,
    pub context_size: usize,
}

impl InputContext {
    pub fn new(context_size: usize, control: InputControlContext, slot: SlotContext, ep0: EndpointContext) -> Self {
        let phys_region = allocate_frame();

        unsafe {
            phys_region.virt.as_mut_ptr::<InputControlContext>().write_volatile(control);
            phys_region.virt.add(context_size as u64).as_mut_ptr::<SlotContext>().write_volatile(slot);
            phys_region.virt.add(context_size as u64 * 2).as_mut_ptr::<EndpointContext>().write_volatile(ep0);
        }

        Self { phys_region, context_size }
    }

    pub fn add_endpoint(&self, endpoint_id: u8, ep_ctx: EndpointContext) {
        assert!(endpoint_id >= 2, "Invalid endpoint DCI");

        self.update_input_control_ctx_add_flags(endpoint_id);
        let offset = self.context_size * (1 + endpoint_id as usize);
        unsafe {
            self.phys_region.virt.add(offset as u64).as_mut_ptr::<EndpointContext>().write_volatile(ep_ctx);
        }
    }
    pub fn update_slot_context_entries_count(&self, entries_count: u8) {
        let slot_context_dword0_ptr = self.phys_region.virt.add(self.context_size as u64).as_mut_ptr::<u32>();
        let mut dword0 = unsafe { slot_context_dword0_ptr.read_volatile() };
        dword0 = (dword0 & !(0x1F << 27)) | ((entries_count as u32) << 27);
        unsafe { slot_context_dword0_ptr.write_volatile(dword0) };
    }
    fn update_input_control_ctx_add_flags(&self, endpoint_id: u8) {
        let add_context_flags_ptr = self.phys_region.virt.add(size_of::<u32>() as u64).as_mut_ptr::<u32>();
        let old = unsafe { add_context_flags_ptr.read_volatile() };
        let clear_ep0 = old & !(1u32 << 1); // no need to reconfigure ep0
        let new = clear_ep0 | (1u32 << endpoint_id);
        unsafe { add_context_flags_ptr.write_volatile(new) };
    }
}
