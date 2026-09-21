use crate::{
    drivers::usb::xhci::{descriptiors::EndpointDescriptorHeader, rings::transfer::TransferRingManager},
    memory::physical::{PhysRegion, allocate_frame},
};

#[derive(Debug)]
pub struct UsbEndpoint {
    pub descriptor: EndpointDescriptorHeader,
    pub endpoint_id: u8, // computed from address
    pub ring: TransferRingManager,
    buffer: Option<PhysRegion>,
}

impl UsbEndpoint {
    pub fn new(endpoint_id: u8, descriptor: EndpointDescriptorHeader) -> Self {
        let is_in = (descriptor.endpoint_address & 0x80) != 0;
        let buffer = if is_in { Some(allocate_frame()) } else { None };
        Self { descriptor, endpoint_id, ring: TransferRingManager::new(), buffer }
    }
    pub fn is_in(&self) -> bool {
        (self.descriptor.endpoint_address & 0x80) != 0
    }
    pub fn is_interrupt(&self) -> bool {
        (self.descriptor.attributes & 0x03) == 0x03
    }
}
