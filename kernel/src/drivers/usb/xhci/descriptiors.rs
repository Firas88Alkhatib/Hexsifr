use alloc::vec::Vec;
// headers
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DeviceDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
    pub bcd_usb: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8, // Maximum packet size for endpoint zero
    pub vendor_id: u16,
    pub product_id: u16,
    pub bcd_device: u16,
    pub manufacturer_index: u8,
    pub product_index: u8,
    pub serial_number_index: u8,
    pub num_configurations: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ConfigDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_index: u8,
    pub attributes: u8,
    pub max_power: u8,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct InterfaceDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8,
    pub interface_index: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EndpointDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HidDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
    pub hid_version: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
    pub descriptor_type_2: u8,
    pub descriptor_length_2: u16,
}

// structs

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    // String = 3,
    // Interface = 4,
    // Endpoint = 5,
    // DeviceQualifier = 6,
    // OtherSpeedConfiguration = 7,
    // InterfacePower = 8,
    // Hid = 0x21,
    Report = 0x22,
    // Physical = 0x23,
    // Hub = 0x29,
}

impl DescriptorType {
    pub const fn value(self, index: u8) -> u16 {
        ((self as u16) << 8) | index as u16
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceDescriptor {
    pub header: InterfaceDescriptorHeader,
    pub endpoints: Vec<EndpointDescriptorHeader>,
    pub hid: Option<HidDescriptorHeader>,
}
#[derive(Debug, Clone)]
pub struct ConfigDescriptor {
    pub header: ConfigDescriptorHeader,
    pub interfaces: Vec<InterfaceDescriptor>,
}
#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub header: DeviceDescriptorHeader,
    pub configs: Vec<ConfigDescriptor>,
}

pub fn parse_config_descriptor(buffer: *mut u8, header: ConfigDescriptorHeader) -> ConfigDescriptor {
    let mut config = ConfigDescriptor { header, interfaces: Vec::new() };

    let mut offset = header.length as usize;
    let total_length = header.total_length as usize;

    let mut current_interface = None;
    const DESC_INTERFACE: u8 = 4;
    const DESC_ENDPOINT: u8 = 5;
    const DESC_HID: u8 = 0x21;

    while offset < total_length {
        let length = unsafe { buffer.add(offset).read() } as usize;
        let descriptor_type = unsafe { buffer.add(offset + 1).read() };
        match descriptor_type {
            DESC_INTERFACE => {
                let interface = unsafe { buffer.add(offset).cast::<InterfaceDescriptorHeader>().read_unaligned() };
                config.interfaces.push(InterfaceDescriptor { header: interface, endpoints: Vec::new(), hid: None });
                current_interface = Some(config.interfaces.len() - 1);
            }
            DESC_ENDPOINT => {
                let endpoint = unsafe { buffer.add(offset).cast::<EndpointDescriptorHeader>().read_unaligned() };
                if let Some(interface_index) = current_interface {
                    config.interfaces[interface_index].endpoints.push(endpoint);
                }
            }
            DESC_HID => {
                let hid = unsafe { buffer.add(offset).cast::<HidDescriptorHeader>().read_unaligned() };
                if let Some(interface_index) = current_interface {
                    config.interfaces[interface_index].hid = Some(hid);
                }
            }
            _ => {}
        }
        if length == 0 {
            break;
        }
        offset += length;
    }

    config
}
