use alloc::vec::Vec;
use x86_64::PhysAddr;

use crate::drivers::usb::xhci::{rings::Ring, trb::Trb};

const ENABLE_SLOT: u32 = 9;
const TRB_TYPE_SHIFT: u32 = 10;
const TRB_ADDRESS_DEVICE: u32 = 11;
const TRB_SLOT_ID_SHIFT: u32 = 24;

#[derive(Debug, Clone, Copy)]
pub struct PendingCommand {
    trb_phys: PhysAddr,
    pub cmd_type: CommandType,
}

#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    EnableSlot { port_id: u8 },
    DisableSlot { slot_id: u8 },
    AddressDevice { slot_id: u8 },
    ConfigureEndpoint { slot_id: u8 },
    EvaluateContext { slot_id: u8 },
    ResetEndpoint { slot_id: u8, endpoint_id: u8 },
    StopEndpoint { slot_id: u8, endpoint_id: u8 },
    SetTrDequeuePointer { slot_id: u8, endpoint_id: u8 },
    ResetDevice { slot_id: u8 },
}
pub struct CommandRing;

impl Ring<CommandRing> {}

pub struct CommandRingManager {
    pub ring: Ring<CommandRing>,
    pending_commands: Vec<PendingCommand>,
}

impl CommandRingManager {
    pub fn new() -> Self {
        Self { ring: Ring::<CommandRing>::new(true, true), pending_commands: Vec::new() }
    }

    fn advance(&mut self) {
        self.ring.next = unsafe { self.ring.next.add(1) };

        if self.ring.next == self.ring.link_trb {
            self.ring.cycle = !self.ring.cycle;
            self.ring.next = self.ring.phys_region.virt.as_mut_ptr::<Trb>();
        }
    }
    fn enqueue(&mut self, mut trb: Trb) -> PhysAddr {
        trb.set_cycle(self.ring.cycle);
        let phys = self.ring.get_next_phys();
        unsafe { self.ring.next.write(trb) };
        self.advance();

        phys
    }
    fn push_pending(&mut self, trb_phys: PhysAddr, cmd_type: CommandType) {
        self.pending_commands.push(PendingCommand { trb_phys, cmd_type });
    }
    pub fn pop_pending(&mut self, trb_phys: PhysAddr) -> Option<PendingCommand> {
        let pos = self.pending_commands.iter().position(|c| c.trb_phys == trb_phys)?;
        Some(self.pending_commands.remove(pos))
    }
    pub fn enqueue_enable_slot(&mut self, port: u8) {
        let control = (ENABLE_SLOT << TRB_TYPE_SHIFT) | self.ring.cycle as u32;

        let trb_phys = self.enqueue(Trb { parameter: 0, status: 0, control });
        self.push_pending(trb_phys, CommandType::EnableSlot { port_id: port });
    }

    pub fn enqueue_address_device(&mut self, input_ctx_phys: PhysAddr, slot_id: u8) {
        let control = ((slot_id as u32) << 24)  // Slot ID
        | (11 << 10)                            // TRB type = Address Device
        | (0 << 9)                              // BSR = 0
        | (self.ring.cycle as u32);

        let trb_phys = self.enqueue(Trb { parameter: input_ctx_phys.as_u64(), status: 0, control });
        self.push_pending(trb_phys, CommandType::AddressDevice { slot_id });
    }

    pub fn enqueue_endpoints_config(&mut self, input_ctx_phys: PhysAddr, slot_id: u8) {
        let control = ((slot_id as u32) << 24) // Slot ID
        | (12 << 10)                           // TRB type = Configure endpoint
        | (0 << 9)                              // BSR = 0
        | (self.ring.cycle as u32);

        let trb_phys = self.enqueue(Trb { parameter: input_ctx_phys.as_u64(), status: 0, control });
        self.push_pending(trb_phys, CommandType::ConfigureEndpoint { slot_id });
    }
}
