use x86_64::PhysAddr;

use super::{Trb, TrbType};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCode {
    Success = 1,
    Invalid = 2,
    TrbError = 5,
    Stall = 6,
    Babble = 7,
    SplitTransaction = 8,
    ShortPacket = 9,
    RingUnderrun = 10,
    RingOverrun = 11,
    VfEventRingFull = 12,
    ParameterError = 13,
    BandwidthError = 14,
    LatencyError = 15,
    PowerError = 16,
    SlotNotEnabled = 17,
    EndpointNotEnabled = 18,
    SlotIdInvalid = 19,
    ContextStateError = 20,
    InvalidEndpoint = 21,
    InvalidDescriptor = 22,
    DeviceError = 23,
    DataBufferError = 24,
    IsochBufferOverrun = 25,
    IsochBufferUnderrun = 26,
    EventLost = 27,
    CommandAborted = 28,
    Underrun = 29,
    Overrun = 30,
    Reserved31 = 31,
    Other(u8),
}

impl From<u8> for CompletionCode {
    fn from(value: u8) -> Self {
        match value {
            1 => CompletionCode::Success,
            2 => CompletionCode::Invalid,
            5 => CompletionCode::TrbError,
            6 => CompletionCode::Stall,
            7 => CompletionCode::Babble,
            8 => CompletionCode::SplitTransaction,
            9 => CompletionCode::ShortPacket,
            10 => CompletionCode::RingUnderrun,
            11 => CompletionCode::RingOverrun,
            12 => CompletionCode::VfEventRingFull,
            13 => CompletionCode::ParameterError,
            14 => CompletionCode::BandwidthError,
            15 => CompletionCode::LatencyError,
            16 => CompletionCode::PowerError,
            17 => CompletionCode::SlotNotEnabled,
            18 => CompletionCode::EndpointNotEnabled,
            19 => CompletionCode::SlotIdInvalid,
            20 => CompletionCode::ContextStateError,
            21 => CompletionCode::InvalidEndpoint,
            22 => CompletionCode::InvalidDescriptor,
            23 => CompletionCode::DeviceError,
            24 => CompletionCode::DataBufferError,
            25 => CompletionCode::IsochBufferOverrun,
            26 => CompletionCode::IsochBufferUnderrun,
            27 => CompletionCode::EventLost,
            28 => CompletionCode::CommandAborted,
            29 => CompletionCode::Underrun,
            30 => CompletionCode::Overrun,
            31 => CompletionCode::Reserved31,
            _ => CompletionCode::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    CommandCompletion(CommandCompletionEvent),
    Transfer(TransferEvent),
    PortStatusChange(PortStatusChangeEvent),
    /// Catch‑all for other event types
    Other {
        trb_type: TrbType,
        raw: Trb,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct CommandCompletionEvent {
    pub completion_code: CompletionCode,
    pub slot_id: u8,
    pub command_trb_phys: PhysAddr, // the 'parameter' field
}

/// Transfer Completion Event (type 32)
#[derive(Debug, Clone, Copy)]
pub struct TransferEvent {
    pub completion_code: CompletionCode,
    pub transfer_length: u32, // bits 23:0 of 'status'
    pub slot_id: u8,
    pub endpoint_id: u8,
    pub trb_phys: PhysAddr, // the 'parameter' field (address of the TRB that completed)
}

/// Port Status Change Event (type 34)
#[derive(Debug, Clone, Copy)]
pub struct PortStatusChangeEvent {
    pub completion_code: CompletionCode,
    pub port_id: u8, // bits 23:16 of 'status'? Actually spec says port number is in bits 23:16 of 'status'
}

impl From<Trb> for Event {
    fn from(trb: Trb) -> Self {
        let trb_type = trb.trb_type();
        let completion_code: CompletionCode = (((trb.status >> 24) & 0xFF) as u8).into();

        match trb_type {
            TrbType::CommandCompletion => Event::CommandCompletion(CommandCompletionEvent {
                completion_code,
                slot_id: ((trb.control >> 24) & 0xFF) as u8,
                command_trb_phys: PhysAddr::new(trb.parameter),
            }),
            TrbType::TransferEvent => Event::Transfer(TransferEvent {
                completion_code,
                transfer_length: trb.status & 0xFFFFFF,
                slot_id: ((trb.control >> 24) & 0xFF) as u8,
                // endpoint_id: ((trb.control >> 16) & 0xFF) as u8,
                endpoint_id: ((trb.control >> 16) & 0x1F) as u8,
                trb_phys: PhysAddr::new(trb.parameter),
            }),
            TrbType::PortStatusChange => Event::PortStatusChange(PortStatusChangeEvent {
                completion_code,
                port_id: ((trb.parameter >> 24) & 0xFF) as u8,
            }),
            _ => Event::Other { trb_type, raw: trb },
        }
    }
}
