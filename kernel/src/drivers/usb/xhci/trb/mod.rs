use x86_64::PhysAddr;

pub mod event;
pub mod setup_packet;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrbType {
    // Transfer (1–8)
    Normal = 1,
    SetupStage = 2,
    DataStage = 3,
    StatusStage = 4,
    Isoch = 5,
    Link = 6,
    EventData = 7,
    NoopTransfer = 8,

    // Command (9–25)
    EnableSlot = 9,
    DisableSlot = 10,
    AddressDevice = 11,
    ConfigureEndpoint = 12,
    EvaluateContext = 13,
    ResetEndpoint = 14,
    StopEndpoint = 15,
    SetTrDequeuePointer = 16,
    ResetDevice = 17,
    ForceEvent = 18,
    NegotiateBandwidth = 19,
    SetLatencyToleranceValue = 20,
    GetPortBandwidth = 21,
    ForceHeader = 22,
    NoopCommand = 23,
    GetExtendedProperty = 24,
    SetExtendedProperty = 25,

    // Event (32–39)
    TransferEvent = 32,
    CommandCompletion = 33,
    PortStatusChange = 34,
    BandwidthRequest = 35,
    Doorbell = 36,
    HostController = 37,
    DeviceNotification = 38,
    MfindexWrap = 39,

    // Catch‑all
    Other(u8),
}

impl From<u8> for TrbType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Normal,
            2 => Self::SetupStage,
            3 => Self::DataStage,
            4 => Self::StatusStage,
            6 => Self::Link,
            8 => Self::NoopTransfer,

            9 => Self::EnableSlot,
            10 => Self::DisableSlot,
            11 => Self::AddressDevice,
            12 => Self::ConfigureEndpoint,
            13 => Self::EvaluateContext,
            14 => Self::ResetEndpoint,
            15 => Self::StopEndpoint,
            16 => Self::SetTrDequeuePointer,
            17 => Self::ResetDevice,
            23 => Self::NoopCommand,

            32 => Self::TransferEvent,
            33 => Self::CommandCompletion,
            34 => Self::PortStatusChange,

            n => Self::Other(n),
        }
    }
}

const TRB_CYCLE: u32 = 1;
const TRB_TYPE_SHIFT: u32 = 10;
const TRB_TOGGLE_CYCLE: u32 = 1 << 1;
const TRB_LINK_TYPE: u32 = 6;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Trb {
    pub parameter: u64,
    pub status: u32,
    pub control: u32,
}

impl Trb {
    pub fn trb_type(&self) -> TrbType {
        (((self.control >> TRB_TYPE_SHIFT) & 0x3F) as u8).into()
    }

    pub fn get_cycle(&self) -> bool {
        (self.control & TRB_CYCLE) != 0
    }
    pub fn set_cycle(&mut self, cycle: bool) {
        self.control = (self.control & !TRB_CYCLE) | (cycle as u32);
    }

    pub fn new_link(phys: PhysAddr, initial_cycle: bool) -> Self {
        let control = (TRB_LINK_TYPE << TRB_TYPE_SHIFT) | TRB_TOGGLE_CYCLE | (initial_cycle as u32);

        Self { parameter: phys.as_u64(), status: 0, control }
    }
}
