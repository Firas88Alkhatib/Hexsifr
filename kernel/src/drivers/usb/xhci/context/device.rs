use core::ops::Add;

use crate::{
    drivers::usb::xhci::context::{endpoint::EndpointContext, slot::SlotContext},
    memory::physical::{PhysRegion, allocate_frame},
};

pub struct DeviceContext {
    pub phys_region: PhysRegion,
}

impl DeviceContext {
    pub fn new() -> Self {
        let phys_region = allocate_frame();
        Self { phys_region }
    }

    /// Returns a mutable pointer to the Slot Context (offset 0x00).
    pub fn slot(&self) -> *mut SlotContext {
        self.phys_region.virt.as_mut_ptr()
    }

    /// Returns a mutable pointer to the Endpoint Context for `endpoint_id` (1‑based).
    /// EP0 = id 1, EP1 OUT = id 2, EP1 IN = id 3, ...
    pub fn endpoint(&self, endpoint_id: usize) -> *mut EndpointContext {
        assert!(endpoint_id >= 1 && endpoint_id <= 31);
        self.phys_region.virt.add(endpoint_id as u64 * 32).as_mut_ptr()
    }
}
