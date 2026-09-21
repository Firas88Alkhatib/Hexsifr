use alloc::{collections::BTreeMap, vec::Vec};

use crate::{
    drivers::usb::xhci::{
        descriptiors::{HidDescriptorHeader, InterfaceDescriptor, InterfaceDescriptorHeader},
        trb::event::TransferEvent,
        usb_device::{
            endpoint::UsbEndpoint,
            hid_report::{HidParser, HidReportDescriptor},
        },
    },
    memory::physical::{PhysRegion, allocate_frame},
};

#[derive(Debug)]
pub struct UsbInterface {
    descriptor_header: InterfaceDescriptorHeader,
    pub endpoints: BTreeMap<u8, UsbEndpoint>,
    hid: Option<HidDescriptorHeader>,  // if class == 0x03
    report_buffer: Option<PhysRegion>, // HID report, lazily fetched
    report_descriptor: Option<HidReportDescriptor>,
}

impl UsbInterface {
    pub fn new(interface_descriptor: InterfaceDescriptor) -> Self {
        let mut endpoints = BTreeMap::new();
        for endpoint_desc in interface_descriptor.endpoints {
            let is_in = (endpoint_desc.endpoint_address & 0x80) != 0;
            let num = endpoint_desc.endpoint_address & 0x0F;
            if num == 0 {
                error!("Trying to add endpoint 0");
                continue; // EP0 — always DCI 1
            }
            let endpoint_id = num * 2 + if is_in { 1 } else { 0 };
            let endpoint = UsbEndpoint::new(endpoint_id, endpoint_desc);
            endpoints.insert(endpoint_id, endpoint);
        }
        Self {
            descriptor_header: interface_descriptor.header,
            endpoints,
            hid: interface_descriptor.hid,
            report_buffer: None,
            report_descriptor: None,
        }
    }

    pub fn set_report_descriptor(&mut self, buffer: PhysRegion, length: u16) {
        let ptr = buffer.virt.as_ptr::<u8>();
        let data = unsafe { core::slice::from_raw_parts(ptr, length as usize) };

        let descriptor = HidParser::parse(&data).expect("Errororeoroeroeoo");
        self.report_descriptor = Some(descriptor)
    }

    pub fn handle_transfer(&mut self, event: TransferEvent) {
        let report_buffer = self.report_buffer.expect("No report buffer");
        let class = self.descriptor_header.interface_class;
        let subclass = self.descriptor_header.interface_subclass;
        let protocol = self.descriptor_header.interface_protocol;

        let reports = &self.report_descriptor.as_ref().expect("Invalid report descriptor").reports;
        let length = reports[&0].input_report_length();

        match (class, subclass, protocol) {
            // HID Boot Keyboard
            (0x03, 0x01, 0x01) => {
                let data = report_buffer.read_as::<KeyboardReport>();
                info!("Report buffer data: {:#?}", data);
            }
            // HID Boot Mouse
            (0x03, 0x01, 0x02) => {
                let data = report_buffer.read_as::<MouseReport>();
                info!("Report buffer data: {:#?}", data);
            }
            _ => {
                todo!();
            }
        };

        let endpoint = self.endpoints.get_mut(&event.endpoint_id).expect("Invalid endpoint id");
        endpoint.ring.normal_transfer(report_buffer, length as u16, true);
    }

    // returns endpoing id
    pub fn setup_interface(&mut self) -> Option<u8> {
        let class = self.descriptor_header.interface_class;
        let subclass = self.descriptor_header.interface_subclass;
        let protocol = self.descriptor_header.interface_protocol;
        match (class, subclass, protocol) {
            // HID Boot Keyboard
            (0x03, 0x01, 0x01) => {
                let endpoint_id = self.setup_keyboard();
                return Some(endpoint_id);
            }
            // HID Boot Mouse
            (0x03, 0x01, 0x02) => {
                let endpoint_id = self.setup_mouse();
                return Some(endpoint_id);
            }
            // HID but not a boot keyboard/mouse
            (0x03, _, _) => {
                todo!();
            }
            // not supported yet
            _ => {
                todo!();
            }
        }
    }

    fn setup_keyboard(&mut self) -> u8 {
        let endpoint = self
            .endpoints
            .values_mut()
            .find(|endpoint| endpoint.is_in() && endpoint.is_interrupt())
            .expect("HID keyboard has no interrupt IN endpoint");

        let report_buffer = allocate_frame();
        self.report_buffer = Some(report_buffer);
        endpoint.ring.normal_transfer(report_buffer, 8, true);
        endpoint.endpoint_id
    }
    fn setup_mouse(&mut self) -> u8 {
        let endpoint = self
            .endpoints
            .values_mut()
            .find(|endpoint| endpoint.is_in() && endpoint.is_interrupt())
            .expect("HID mouse has no interrupt IN endpoint");

        let report_buffer = allocate_frame();
        self.report_buffer = Some(report_buffer);
        endpoint.ring.normal_transfer(report_buffer, 4, true);
        endpoint.endpoint_id
    }
}
#[derive(Debug)]
pub struct InterfaceManager {
    pub interfaces: Vec<UsbInterface>,
    endpoint_to_interface: BTreeMap<u8, usize>,
}

impl InterfaceManager {
    pub fn new() -> Self {
        Self { interfaces: Vec::new(), endpoint_to_interface: BTreeMap::new() }
    }
    pub fn add_interface(&mut self, interface: UsbInterface) {
        let interface_idx = self.interfaces.len();
        for endpoint_id in interface.endpoints.keys() {
            let old_value = self.endpoint_to_interface.insert(endpoint_id.clone(), interface_idx);

            if old_value.is_some() {
                panic!("endpoint with the same id already exist");
            };
        }
        self.interfaces.push(interface);
    }

    pub fn get_interface_mut(&mut self, index: u8) -> &mut UsbInterface {
        self.interfaces.get_mut(index as usize).expect("Interface not found")
    }

    pub fn get_interface_by_endpoint_id_mut(&mut self, endpoint_id: u8) -> &mut UsbInterface {
        let interface_idx = self.endpoint_to_interface.get_mut(&endpoint_id).expect("endpoint not found");
        &mut self.interfaces[interface_idx.clone()]
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct KeyboardReport {
    pub modifiers: u8,
    pub reserved: u8,
    pub keys: [u8; 6],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MouseReport {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8,
}
