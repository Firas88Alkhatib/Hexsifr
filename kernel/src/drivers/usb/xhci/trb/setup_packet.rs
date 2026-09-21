use crate::drivers::usb::xhci::descriptiors::DescriptorType;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbDirection {
    Out = 0x00, // host → device
    In = 0x80,  // device → host
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRequestType {
    Standard = 0x00,
    Class = 0x20,
    Vendor = 0x40,
    Reserved = 0x60,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRecipient {
    Device = 0x00,
    Interface = 0x01,
    Endpoint = 0x02,
    Other = 0x03,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbRequest {
    GetStatus = 0x00,
    ClearFeature = 0x01,
    SetFeature = 0x03,
    SetAddress = 0x05,
    GetDescriptor = 0x06,
    SetDescriptor = 0x07,
    GetConfiguration = 0x08,
    SetConfiguration = 0x09,
    GetInterface = 0x0A,
    SetInterface = 0x0B,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SetupPacket {
    /// Bit 7:    Direction (0 = host-to-device, 1 = device-to-host)
    /// Bits 6:5: Type (0 = standard, 1 = class, 2 = vendor, 3 = reserved)
    /// Bits 4:0: Recipient (0 = device, 1 = interface, 2 = endpoint, ...)
    pub request_type: u8,

    /// Request code (e.g., 0x06 for GET_DESCRIPTOR, 0x09 for SET_CONFIGURATION)
    pub request: u8,

    /// Value — meaning depends on the request:
    ///   GET_DESCRIPTOR: high byte = descriptor type, low byte = descriptor index
    ///   SET_CONFIGURATION: low byte = configuration value
    ///   SET_ADDRESS: low byte = device address
    pub value: u16,

    /// Index — meaning depends on the request:
    ///   Usually interface number or endpoint number, 0 otherwise
    pub index: u16,

    /// Number of bytes to transfer in the data stage
    pub length: u16,
}

impl SetupPacket {
    pub const fn new(
        dir: UsbDirection,
        kind: UsbRequestType,
        recip: UsbRecipient,
        request: UsbRequest,
        value: u16,
        index: u16,
        length: u16,
    ) -> Self {
        Self { request_type: Self::request_type(dir, kind, recip), request: request as u8, value, index, length }
    }

    pub const fn request_type(dir: UsbDirection, kind: UsbRequestType, recip: UsbRecipient) -> u8 {
        (dir as u8) | (kind as u8) | (recip as u8)
    }

    pub const fn get_descriptor(desc: DescriptorType, index: u8, length: u16) -> Self {
        Self::new(
            UsbDirection::In,
            UsbRequestType::Standard,
            UsbRecipient::Device,
            UsbRequest::GetDescriptor,
            desc.value(index),
            index as u16,
            length,
        )
    }

    pub const fn set_configuration(config_value: u8) -> Self {
        Self::new(
            UsbDirection::Out,
            UsbRequestType::Standard,
            UsbRecipient::Device,
            UsbRequest::SetConfiguration,
            config_value as u16,
            0,
            0,
        )
    }

    pub const fn get_hid_report_descriptor(interface: u8, length: u16) -> Self {
        Self::new(
            UsbDirection::In,
            UsbRequestType::Standard,
            UsbRecipient::Interface,
            UsbRequest::GetDescriptor,
            DescriptorType::Report.value(0),
            interface as u16,
            length,
        )
    }
}
