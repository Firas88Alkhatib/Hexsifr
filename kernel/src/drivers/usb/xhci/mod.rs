mod context;
mod dcbaa;
mod descriptiors;
mod doorbells;
mod extended_capabilities;
mod interrupter;
mod mmio;
mod msix;
mod ports;
mod rings;
mod trb;
mod usb_device;

use alloc::{format, vec::Vec};
use core::ops::Add;
use spin::Once;
use x86_64::VirtAddr;

use crate::{
    drivers::{
        pci::pci_class::PciDevice,
        usb::xhci::{
            dcbaa::Dcbaa,
            doorbells::DoorbellsManager,
            extended_capabilities::get_ports_versions_protocols,
            interrupter::{Interrupter, InterrupterRegs},
            mmio::{CapabilityRegs, Mmio, OperationalRegs, RuntimeRegs},
            msix::MsixCap,
            ports::PortsManager,
            rings::command::{CommandRingManager, CommandType},
            trb::event::{CommandCompletionEvent, CompletionCode, Event, PortStatusChangeEvent, TransferEvent},
            usb_device::DeviceManger,
        },
    },
    memory::mapper::map_mmio,
};

static XHCI_CONTROLLER: Once<Xhci> = Once::new();

pub fn xhci_init(pci_device: PciDevice) {
    XHCI_CONTROLLER.call_once(|| Xhci::new(pci_device));
    get_xhci().init();
}

pub fn get_xhci() -> &'static mut Xhci {
    unsafe { &mut *XHCI_CONTROLLER.as_mut_ptr() }
}

pub fn handle_interrupt() {
    if !XHCI_CONTROLLER.is_completed() {
        info!("not initiallized");
        return;
    }
    get_xhci().handle_interrupt();
}

pub struct Xhci {
    pci_device: PciDevice,
    bar_virt: VirtAddr,
    cap: Mmio<CapabilityRegs>,
    op: Mmio<OperationalRegs>,
    runtime: Mmio<RuntimeRegs>,
    interrupters: Vec<Interrupter>,
    port_manager: PortsManager,
    doorbells: DoorbellsManager,
    device_manager: DeviceManger,

    dcbaa: Dcbaa,
    command_ring: CommandRingManager,
}

// TODO figure out a better way
unsafe impl Send for Xhci {}
unsafe impl Sync for Xhci {}

impl Xhci {
    pub fn new(pci_device: PciDevice) -> Self {
        let bar_phys = pci_device.get_bar_base(0).expect("Failed to get xhci bar address");
        let bar_virt = map_mmio(bar_phys, 0x10000); // xHCI region size , we need to figure out how to know its size

        let cap = Mmio::<CapabilityRegs>::new(bar_virt);

        let op_addr = bar_virt.add(cap.cap_length.read() as u64);
        let op = Mmio::<OperationalRegs>::new(op_addr);

        // TODO use interrupter per cpu
        // Also each interrupter should own its own event ring and each event ring should own erst
        let runtime = Mmio::<RuntimeRegs>::new(bar_virt.add(cap.rts_off.read() as u64));
        let interrupters_base = runtime.addr().add(size_of::<RuntimeRegs>() as u64);
        let max_interrupters = cap.get_max_interrupters();

        let interrupters: Vec<Interrupter> = (interrupters_base..)
            .step_by(size_of::<InterrupterRegs>())
            .take(max_interrupters as usize)
            .map(|addr| Interrupter::new(addr))
            .collect();

        let doorbells_base = bar_virt.add(cap.db_off.read() as u64);
        let doorbells = DoorbellsManager::new(doorbells_base, cap.get_max_slots());

        const PORTS_OFFSET: u64 = 0x400;
        let ports_base = op.addr().add(PORTS_OFFSET);
        let xecp_offset = cap.get_extended_capabilities_offset();
        let ports_protocols_map = get_ports_versions_protocols(bar_virt, xecp_offset);
        let port_manager = PortsManager::new(ports_base, ports_protocols_map);

        let dcbaa = Dcbaa::new(cap.get_max_scratchpads());
        let command_ring = CommandRingManager::new();
        let device_manager = DeviceManger::new(cap.get_context_size());

        Self {
            pci_device,
            bar_virt,
            cap,
            op,
            port_manager,
            device_manager,
            runtime,
            interrupters,
            doorbells,
            command_ring,
            dcbaa,
        }
    }

    fn setup_interrupts(&self) {
        let binding = self.pci_device.capabilities();
        let (_, msix_phys) = binding.iter().find(|(header, _)| header.id == 0x11).expect("MSI-X not supported");
        let msix = MsixCap::new(msix_phys.clone(), self.bar_virt);
        msix.write_entry();
        msix.enable();
    }

    pub fn acknowledge_irq(&self, interrupter: &Interrupter) {
        self.op.eoi();
        interrupter.eoi();
    }

    pub fn init(&mut self) {
        self.setup_interrupts();
        self.op.stop();
        self.op.reset();
        self.op.enable_all_notifications(); // mostly used for debugging
        self.op.config_max_slots(self.cap.get_max_slots());
        self.op.set_command_ring(&self.command_ring.ring);
        self.op.set_dcbaa(&self.dcbaa);
        let interrupter = &self.interrupters[0];
        interrupter.init();
        self.acknowledge_irq(interrupter);
        self.op.start_and_enable();
        self.port_manager.reset_connected_ports();
    }
    fn on_command_completion(&mut self, event: CommandCompletionEvent) {
        let pending_command = self
            .command_ring
            .pop_pending(event.command_trb_phys)
            .expect(&format!("No assosiated pending command for event : {:#?}", event));

        if event.completion_code != CompletionCode::Success {
            info!("command completion failed for pending command {:#?}", pending_command);
            return;
        }

        match pending_command.cmd_type {
            CommandType::EnableSlot { port_id } => self.address_device(port_id, event.slot_id),
            CommandType::AddressDevice { slot_id } => {
                self.device_manager.get_device_mut(slot_id).request_device_descriptor_info();
            }
            CommandType::ConfigureEndpoint { slot_id } => {
                self.device_manager.get_device_mut(slot_id).on_config_endpoint();
            }
            CommandType::DisableSlot { slot_id } => todo!(),
            CommandType::EvaluateContext { slot_id } => todo!(),
            CommandType::ResetEndpoint { slot_id, endpoint_id } => todo!(),
            CommandType::StopEndpoint { slot_id, endpoint_id } => todo!(),
            CommandType::SetTrDequeuePointer { slot_id, endpoint_id } => todo!(),
            CommandType::ResetDevice { slot_id } => todo!(),
        }
    }
    pub fn process_events(&mut self, interrupter_index: usize) {
        let interrupter = &mut self.interrupters[interrupter_index];

        for event in interrupter.dequeue_events() {
            match event {
                Event::CommandCompletion(cmd_event) => self.on_command_completion(cmd_event),
                Event::Transfer(transfer_event) => self.on_transfer(transfer_event),
                Event::PortStatusChange(psc_event) => self.on_port_status_change(psc_event),
                Event::Other { trb_type, raw } => {
                    info!("Other event received : trb type {:#?} - raw {:#?}", trb_type, raw);
                }
            }
        }
    }
    fn on_port_status_change(&mut self, event: PortStatusChangeEvent) {
        if event.completion_code != CompletionCode::Success {
            info!("Port status change failed: {:#?}", event);
            return;
        }
        self.enable_slot(event.port_id);
    }
    fn on_transfer(&mut self, event: TransferEvent) {
        if event.completion_code != CompletionCode::Success {
            panic!("transfer event failed");
        }
        let device = self.device_manager.get_device_mut(event.slot_id);
        let input_ctx_phys = device.on_transfer(event);

        if let Some(phys) = input_ctx_phys {
            self.command_ring.enqueue_endpoints_config(phys, event.slot_id);
            self.doorbells.ring_command_doorbell();
        }
    }
    fn enable_slot(&mut self, port_id: u8) {
        self.command_ring.enqueue_enable_slot(port_id);
        self.doorbells.ring_command_doorbell();
    }

    pub fn handle_interrupt(&mut self) {
        self.process_events(0);
        self.acknowledge_irq(&self.interrupters[0]);
    }
    pub fn address_device(&mut self, port_id: u8, slot_id: u8) {
        let port = self.port_manager.ports.get_mut(&port_id).expect("Invalid port");
        let doorbell_addr = self.doorbells.get_doorbell(slot_id).addr();

        let (dev_ctx_phys, input_ctx_phys) = self.device_manager.add_device(port, slot_id, doorbell_addr);
        self.dcbaa.set(slot_id, dev_ctx_phys);
        self.command_ring.enqueue_address_device(input_ctx_phys, slot_id);
        self.doorbells.ring_command_doorbell();
    }
}
