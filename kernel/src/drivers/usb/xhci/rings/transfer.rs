use alloc::vec::Vec;
use x86_64::PhysAddr;

use crate::{
    drivers::usb::xhci::{
        descriptiors::DescriptorType,
        rings::{Ring, TRB_COUNT},
        trb::{Trb, setup_packet::SetupPacket},
    },
    memory::physical::PhysRegion,
};

// TRB types
const TRB_NORMAL: u32 = 1;
const TRB_SETUP_STAGE: u32 = 2;
const TRB_DATA_STAGE: u32 = 3;
const TRB_STATUS_STAGE: u32 = 4;

const TRB_TYPE_SHIFT: u32 = 10;
const TRB_IOC_BIT: u32 = 1 << 5; // Interrupt On Completion
const TRB_IDT_BIT: u32 = 1 << 6; // Immediate Data (Setup)
const TRB_DIR_IN_BIT: u32 = 1 << 16; // Direction (Data/Status)
const TRB_TRT_SHIFT: u32 = 16; // Transfer Type (Setup)

#[derive(Debug, Clone, Copy)]
pub struct PendingTransfer {
    pub trb_phys: PhysAddr,
    pub kind: TransferType,
    pub buffer: Option<PhysRegion>,
    pub data_length: u16,
    pub interface_number: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    RequestDescriptor(DescriptorType),
    RequestDeviceDescriptorInfo,
    RequestConfigDescriptorInfo,
    SetConfiguration,
    Normal,
}
#[derive(Debug)]
pub struct TransferRing;

impl Ring<TransferRing> {}

#[derive(Debug)]
pub struct TransferRingManager {
    pub ring: Ring<TransferRing>,
    pending: Vec<PendingTransfer>,
}

impl TransferRingManager {
    pub fn new() -> Self {
        Self { ring: Ring::<TransferRing>::new(true, true), pending: Vec::new() }
    }

    fn advance(&mut self) {
        self.ring.next = unsafe { self.ring.next.add(1) };
        if self.ring.next == self.ring.link_trb {
            unsafe { (*self.ring.link_trb).set_cycle(self.ring.cycle) };
            self.ring.cycle = !self.ring.cycle;
            self.ring.next = self.ring.phys_region.virt.as_mut_ptr::<Trb>();
        }
    }

    fn enqueue(&mut self, trb: Trb) -> PhysAddr {
        // trb.set_cycle(self.ring.cycle);
        let phys = self.ring.get_next_phys();
        unsafe { self.ring.next.write_volatile(trb) };
        self.advance();
        phys
    }

    pub fn take_pending(&mut self, trb_phys: PhysAddr) -> Option<PendingTransfer> {
        let pos = self.pending.iter().position(|t| t.trb_phys == trb_phys)?;
        Some(self.pending.remove(pos))
    }

    /// Enqueues a control transfer. Returns the physical address of the
    /// Setup TRB, which the Transfer Event will reference.
    pub fn control_transfer(
        &mut self,
        setup: SetupPacket,
        kind: TransferType,
        buffer: Option<PhysRegion>,
        interface: Option<u8>,
    ) -> PhysAddr {
        let status_trb_phys = self.enqueue_control(setup, buffer.and_then(|b| Some(b.phys)));

        self.pending.push(PendingTransfer {
            trb_phys: status_trb_phys,
            kind,
            buffer,
            data_length: setup.length,
            interface_number: interface,
        });
        status_trb_phys
    }

    fn enqueue_control(&mut self, setup: SetupPacket, buffer_phys: Option<PhysAddr>) -> PhysAddr {
        let length = setup.length as u32;
        let dir_in = (setup.request_type & 0x80) != 0;
        // TRT: 0 = no data, 2 = OUT data, 3 = IN data
        let trt = (length > 0) as u32 * (2 + dir_in as u32);

        let cycle = self.ring.cycle as u32;

        // Setup stage — packs the 8‑byte setup packet directly into the TRB.
        let d0 = (setup.request_type as u32) | ((setup.request as u32) << 8) | ((setup.value as u32) << 16);
        let d1 = (setup.index as u32) | (length << 16);

        self.enqueue(Trb {
            parameter: ((d1 as u64) << 32) | d0 as u64,
            status: 8,
            control: (TRB_SETUP_STAGE << TRB_TYPE_SHIFT) | (trt << TRB_TRT_SHIFT) | TRB_IDT_BIT | cycle,
        });

        // Data stage (skipped when length == 0).
        if length > 0 {
            let buf = buffer_phys.expect("data stage requires a buffer");
            self.enqueue(Trb {
                parameter: buf.as_u64(),
                status: length,
                control: (TRB_DATA_STAGE << TRB_TYPE_SHIFT) | ((dir_in as u32) << 16) | cycle,
            });
        }

        // Status stage — direction opposite of data.
        let status_trb_phys = self.enqueue(Trb {
            parameter: 0,
            status: 0,
            control: (TRB_STATUS_STAGE << TRB_TYPE_SHIFT) | (((!dir_in) as u32) << 16) | TRB_IOC_BIT | cycle,
        });

        status_trb_phys
    }

    /// Enqueues a Normal TRB for bulk or interrupt endpoints.
    /// Returns the physical address of the Normal TRB.
    pub fn normal_transfer(&mut self, buffer: PhysRegion, length: u16, ioc: bool) -> PhysAddr {
        let control = (TRB_NORMAL << TRB_TYPE_SHIFT) | if ioc { TRB_IOC_BIT } else { 0 } | (self.ring.cycle as u32);
        let trb_phys = self.enqueue(Trb { parameter: buffer.phys.as_u64(), status: length as u32, control });
        self.pending.push(PendingTransfer {
            trb_phys,
            kind: TransferType::Normal,
            buffer: Some(buffer),
            data_length: length,
            interface_number: None,
        });
        trb_phys
    }
}
