use core::ops::Add;
pub mod interface;
pub mod hid_report;
pub mod endpoint;


use crate::drivers::usb::xhci::{
    context::{
        device::DeviceContext,
        endpoint::EndpointContext,
        input::{InputContext, InputControlContext},
        slot::SlotContext,
    },
    descriptiors::{
        ConfigDescriptorHeader, DescriptorType, DeviceDescriptor, DeviceDescriptorHeader, parse_config_descriptor,
    },
    doorbells::Doorbell,
    mmio::Mmio,
    ports::Port,
    rings::transfer::{TransferRingManager, TransferType},
    trb::{event::TransferEvent, setup_packet::SetupPacket},
    usb_device::interface::{InterfaceManager, UsbInterface},
};
use crate::memory::physical::{PhysRegion, allocate_frame};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use x86_64::{PhysAddr, VirtAddr};

pub struct UsbDevice {
    port: Port,
    slot_id: u8,
    doorbell: Mmio<Doorbell>,
    input_ctx: InputContext,
    control_ring: TransferRingManager,
    control_buffer: PhysRegion,
    device_descriptor: Option<DeviceDescriptor>,
    interfaces: InterfaceManager,
}

impl UsbDevice {
    pub fn on_transfer(&mut self, event: TransferEvent) -> Option<PhysAddr> {
        if event.endpoint_id == 1 {
            return self.handle_control_transfer(event);
        }

        self.interfaces.get_interface_by_endpoint_id_mut(event.endpoint_id).handle_transfer(event);
        self.doorbell.ring(event.endpoint_id);

        None
    }
    fn handle_control_transfer(&mut self, event: TransferEvent) -> Option<PhysAddr> {
        let pending_transfer = self.control_ring.take_pending(event.trb_phys).expect("Invalid pending transfer");

        match pending_transfer.kind {
            TransferType::RequestDeviceDescriptorInfo => {
                self.on_device_descriptor_info(pending_transfer.buffer.expect("Invalid buffer"));
            }
            TransferType::RequestDescriptor(DescriptorType::Device) => {
                self.on_device_descriptor(pending_transfer.buffer.expect("Invalid buffer"));
            }
            TransferType::RequestConfigDescriptorInfo => {
                self.on_config_descriptor_info(pending_transfer.buffer.expect("Invalid buffer"));
            }
            TransferType::RequestDescriptor(DescriptorType::Configuration) => {
                self.on_config_descriptor(pending_transfer.buffer.expect("Invalid buffer"));
            }
            TransferType::RequestDescriptor(DescriptorType::Report) => {
                let interface_number = pending_transfer.interface_number.expect("Interface number not set");
                let length = pending_transfer.data_length;
                let buffer = pending_transfer.buffer.expect("Invalid buffer");
                self.interfaces.get_interface_mut(interface_number).set_report_descriptor(buffer, length);
            }
            TransferType::SetConfiguration => {
                self.on_set_configuration();
                return Some(self.input_ctx.phys_region.phys);
            }
            _ => todo!(),
        }
        None
    }

    pub fn request_device_descriptor_info(&mut self) {
        self.control_buffer.fill_zero();
        const LENGHT: u16 = 8;
        let setup = SetupPacket::get_descriptor(DescriptorType::Device, 0, LENGHT);
        let kind = TransferType::RequestDeviceDescriptorInfo;
        self.control_ring.control_transfer(setup, kind, Some(self.control_buffer), None);
        self.doorbell.ring(1);
    }
    pub fn on_device_descriptor_info(&mut self, buffer: PhysRegion) {
        let max_packet_size0 = unsafe { buffer.virt.add(7).as_ptr::<u8>().read_volatile() };

        let port_speed = self.port.port_speed();
        let max_packet_size0 = port_speed.get_max_packet_size_for_ep0(max_packet_size0);

        if max_packet_size0 != port_speed.max_packet_size() {
            todo!("We need to readdress the device with the new max packet size");
            // return;
        }

        let length = unsafe { buffer.virt.add(7).as_ptr::<u8>().read_volatile() };
        self.request_device_desciptor(length);
        self.doorbell.ring(1);
    }
    pub fn request_device_desciptor(&mut self, length: u8) {
        self.control_buffer.fill_zero();
        let setup = SetupPacket::get_descriptor(DescriptorType::Device, 0, length as u16);
        let kind = TransferType::RequestDescriptor(DescriptorType::Device);
        self.control_ring.control_transfer(setup, kind, Some(self.control_buffer), None);
        self.doorbell.ring(1);
    }
    pub fn on_device_descriptor(&mut self, buffer: PhysRegion) {
        let header = buffer.read_as::<DeviceDescriptorHeader>();

        self.device_descriptor = Some(DeviceDescriptor { header, configs: Vec::new() });

        self.request_config_descriptor_info();
        self.doorbell.ring(1);
    }
    pub fn request_config_descriptor_info(&mut self) {
        self.control_buffer.fill_zero();
        const LENGHT: u16 = 9;
        let setup = SetupPacket::get_descriptor(DescriptorType::Configuration, 0, LENGHT);
        let kind = TransferType::RequestConfigDescriptorInfo;
        self.control_ring.control_transfer(setup, kind, Some(self.control_buffer), None);
        self.doorbell.ring(1);
    }
    pub fn on_config_descriptor_info(&mut self, buffer: PhysRegion) {
        let total_length = unsafe { buffer.virt.add(2).as_ptr::<u16>().read_volatile() };
        self.request_config_descriptor(total_length);
        self.doorbell.ring(1);
    }
    pub fn request_config_descriptor(&mut self, total_length: u16) {
        self.control_buffer.fill_zero();
        let setup = SetupPacket::get_descriptor(DescriptorType::Configuration, 0, total_length);
        let kind = TransferType::RequestDescriptor(DescriptorType::Configuration);
        self.control_ring.control_transfer(setup, kind, Some(self.control_buffer), None);
        self.doorbell.ring(1);
    }
    pub fn on_config_descriptor(&mut self, buffer: PhysRegion) {
        let config_descriptor_header = buffer.read_as::<ConfigDescriptorHeader>();
        let config = parse_config_descriptor(buffer.virt.as_mut_ptr::<u8>(), config_descriptor_header);

        for interface_descriptor in &config.interfaces {
            self.interfaces.add_interface(UsbInterface::new(interface_descriptor.clone()));

            if let Some(hid_descriptor) = interface_descriptor.hid {
                self.request_report_descriptor(
                    interface_descriptor.header.interface_number,
                    hid_descriptor.descriptor_length_2,
                )
            }
        }

        self.device_descriptor.as_mut().expect("Device descriptor is not set").configs.push(config);
        self.set_configuration(config_descriptor_header.configuration_value);
        self.doorbell.ring(1);
    }
    pub fn request_report_descriptor(&mut self, interface_number: u8, length: u16) {
        let setup = SetupPacket::get_hid_report_descriptor(interface_number, length);
        let kind = TransferType::RequestDescriptor(DescriptorType::Report);
        self.control_ring.control_transfer(setup, kind, Some(self.control_buffer), Some(interface_number));
        self.doorbell.ring(1);
    }

    pub fn set_configuration(&mut self, config_value: u8) {
        let setup = SetupPacket::set_configuration(config_value);
        let kind = TransferType::SetConfiguration;
        self.control_ring.control_transfer(setup, kind, None, None);
        self.doorbell.ring(1);
    }
    pub fn on_set_configuration(&mut self) {
        let port_speed = self.port.port_speed();

        let highest_dci =
            self.interfaces.interfaces.iter().flat_map(|i| i.endpoints.keys().copied()).max().unwrap_or(1).max(1);

        self.input_ctx.update_slot_context_entries_count(highest_dci);

        self.interfaces.interfaces.iter().flat_map(|interface| interface.endpoints.values()).for_each(|endpoint| {
            let ep_ctx = EndpointContext::for_interrupt_in(
                endpoint.ring.ring.phys_region.phys,
                endpoint.descriptor.max_packet_size,
                port_speed.get_interval(endpoint.descriptor.interval),
            );

            self.input_ctx.add_endpoint(endpoint.endpoint_id, ep_ctx);
        });
    }
    pub fn on_config_endpoint(&mut self) {
        for interface in self.interfaces.interfaces.iter_mut() {
            if let Some(endpoint_id) = interface.setup_interface() {
                self.doorbell.ring(endpoint_id);
            }
        }
    }
}

pub struct DeviceManger {
    // Slot id -> Device
    devices: BTreeMap<u8, UsbDevice>,
    context_size: usize,
}

impl DeviceManger {
    pub fn new(context_size: usize) -> Self {
        Self { devices: BTreeMap::new(), context_size }
    }
    pub fn add_device(&mut self, port: &Port, slot_id: u8, doorbell_addr: VirtAddr) -> (PhysAddr, PhysAddr) {
        let doorbell = Mmio::<Doorbell>::new(doorbell_addr);
        let port = Port::new(port.revision, port.addr, port.port_id);
        let port_speed = port.port_speed();

        let input_ctrl_ctx = InputControlContext::for_address_device();
        let slot_ctx = SlotContext::for_address_device(port_speed, port.port_id);
        let control_ring = TransferRingManager::new();

        let ep0_ctx =
            EndpointContext::for_control_ep0(control_ring.ring.phys_region.phys, port_speed.max_packet_size());
        let input_ctx = InputContext::new(self.context_size, input_ctrl_ctx, slot_ctx, ep0_ctx);
        let input_ctx_phys = input_ctx.phys_region.phys;

        let device = UsbDevice {
            port,
            slot_id,
            doorbell,
            input_ctx,
            control_ring,
            control_buffer: allocate_frame(),
            device_descriptor: None,
            interfaces: InterfaceManager::new(),
        };

        self.devices.insert(slot_id, device);

        let dev_ctx_phys = DeviceContext::new().phys_region.phys;
        (dev_ctx_phys, input_ctx_phys)
    }

    pub fn get_device_mut(&mut self, slot_id: u8) -> &mut UsbDevice {
        self.devices.get_mut(&slot_id).expect("Invalid slot id")
    }
}
