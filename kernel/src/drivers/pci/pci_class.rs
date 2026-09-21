use core::ops::Add;

use alloc::vec::Vec;
use x86_64::PhysAddr;

use crate::memory::phys_to_virt;

#[derive(Debug, Clone)]
pub struct PciDevice {
    pub header: PciConfigHeader,
    pub config_base: PhysAddr,
}
impl PciDevice {
    pub fn get_bar_base(&self, index: usize) -> Option<PhysAddr> {
        let bars = [
            self.header.bar0,
            self.header.bar1,
            self.header.bar2,
            self.header.bar3,
            self.header.bar4,
            self.header.bar5,
        ];
        let bar = *bars.get(index)?;

        if (bar & 1) != 0 {
            return None;
        }

        let base = (bar & 0xfffffff0) as u64;
        let is_64 = bar & 0x6 == 0x4;
        if !is_64 {
            return Some(PhysAddr::new(base));
        }

        let next_bar = *bars.get(index + 1)? as u64;
        Some(PhysAddr::new(base | next_bar << 32))
    }
    pub fn capabilities(&self) -> Vec<(CapabilityHeader, PhysAddr)> {
        let mut capabilities = Vec::new();
        let mut cap_ptr = self.header.capabilities_ptr;

        while cap_ptr != 0 {
            let cap_phys = self.config_base.add(cap_ptr as u64);
            let cap_header = unsafe { phys_to_virt::<CapabilityHeader>(cap_phys.as_u64()).read_volatile() };
            capabilities.push((cap_header, cap_phys));
            cap_ptr = cap_header.next;
        }

        capabilities
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PciConfigHeader {
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub class_prog_if: u8,
    pub class_sub: u8,
    pub class_base: u8,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,
    pub bar0: u32,
    pub bar1: u32,
    pub bar2: u32,
    pub bar3: u32,
    pub bar4: u32,
    pub bar5: u32,
    pub cis_ptr: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub rom_base: u32,
    pub capabilities_ptr: u8,
    pub reserved: [u8; 7],
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub min_gnt: u8,
    pub max_lat: u8,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct CapabilityHeader {
    pub id: u8,
    pub next: u8,
}

#[derive(Debug, Default)]
pub struct MassStorage {
    pub scsi: Vec<PciDevice>,
    pub ide: Ide,
    pub floppy: Vec<PciDevice>,
    pub ipi: Vec<PciDevice>,
    pub raid: Vec<PciDevice>,
    pub ata: Vec<PciDevice>,
    pub sata: Sata,
    pub sas: Vec<PciDevice>,
    pub nvme: Vec<PciDevice>,
    pub ufs: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Ide {
    pub isa_compatibility: Vec<PciDevice>,
    pub pci_native: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Sata {
    pub vendor_specific: Vec<PciDevice>,
    pub ahci: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Network {
    pub ethernet: Vec<PciDevice>,
    pub token_ring: Vec<PciDevice>,
    pub fddi: Vec<PciDevice>,
    pub atm: Vec<PciDevice>,
    pub isdn: Vec<PciDevice>,
    pub world_fip: Vec<PciDevice>,
    pub picmg: Vec<PciDevice>,
    pub infini_band: Vec<PciDevice>,
    pub fabric: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Display {
    pub vga: Vec<PciDevice>,
    pub xga: Vec<PciDevice>,
    pub three_d: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Multimedia {
    pub video: Vec<PciDevice>,
    pub audio: Vec<PciDevice>,
    pub telephony: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Memory {
    pub ram: Vec<PciDevice>,
    pub flash: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Bridge {
    pub host: Vec<PciDevice>,
    pub isa: Vec<PciDevice>,
    pub eisa: Vec<PciDevice>,
    pub mca: Vec<PciDevice>,
    pub pci_to_pci: Vec<PciDevice>,
    pub pcmcia: Vec<PciDevice>,
    pub nu_bus: Vec<PciDevice>,
    pub card_bus: Vec<PciDevice>,
    pub raceway: Vec<PciDevice>,
    pub pci_to_pci_non_transparent: Vec<PciDevice>,
    pub infini_band: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct SimpleCommunication {
    pub serial: Vec<PciDevice>,
    pub parallel: Vec<PciDevice>,
    pub multiport_serial: Vec<PciDevice>,
    pub modem: Vec<PciDevice>,
    pub gpib: Vec<PciDevice>,
    pub smart_card: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct BaseSystemPeripheral {
    pub pic: Vec<PciDevice>,
    pub dma: Vec<PciDevice>,
    pub timer: Vec<PciDevice>,
    pub rtc: Vec<PciDevice>,
    pub hot_plug: Vec<PciDevice>,
    pub sd_host: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Input {
    pub keyboard: Vec<PciDevice>,
    pub digitizer: Vec<PciDevice>,
    pub mouse: Vec<PciDevice>,
    pub scanner: Vec<PciDevice>,
    pub gameport: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Docking {
    pub generic: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Processor {
    pub x86: Vec<PciDevice>,
    pub x86_64: Vec<PciDevice>,
    pub alpha: Vec<PciDevice>,
    pub power_pc: Vec<PciDevice>,
    pub mips: Vec<PciDevice>,
    pub co_processor: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct SerialBus {
    pub fire_wire: Vec<PciDevice>,
    pub access_bus: Vec<PciDevice>,
    pub ssa: Vec<PciDevice>,
    pub usb: Usb,
    pub fibre_channel: Vec<PciDevice>,
    pub sm_bus: Vec<PciDevice>,
    pub infini_band: Vec<PciDevice>,
    pub ipmi_smic: Vec<PciDevice>,
    pub sercos: Vec<PciDevice>,
    pub can_bus: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Usb {
    pub uhci: Vec<PciDevice>,
    pub ohci: Vec<PciDevice>,
    pub ehci: Vec<PciDevice>,
    pub xhci: Vec<PciDevice>,
    pub usb_device: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Wireless {
    pub bluetooth: Vec<PciDevice>,
    pub wifi: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct IntelligentIo {
    pub i2o: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct Encryption {
    pub network_crypto: Vec<PciDevice>,
    pub entertainment_crypto: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct DataAcquisition {
    pub daq: Vec<PciDevice>,
    pub dsp: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct ProcessingAccelerator {
    pub generic: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

#[derive(Debug, Default)]
pub struct PciDevices {
    pub unclassified: Vec<PciDevice>,
    pub mass_storage: MassStorage,
    pub network: Network,
    pub display: Display,
    pub multimedia: Multimedia,
    pub memory: Memory,
    pub bridge: Bridge,
    pub simple_communication: SimpleCommunication,
    pub base_system_peripheral: BaseSystemPeripheral,
    pub input: Input,
    pub docking: Docking,
    pub processor: Processor,
    pub serial_bus: SerialBus,
    pub wireless: Wireless,
    pub intelligent_io: IntelligentIo,
    pub satellite: Vec<PciDevice>,
    pub encryption: Encryption,
    pub data_acquisition: DataAcquisition,
    pub processing_accelerator: ProcessingAccelerator,
    pub non_essential_instrumentation: Vec<PciDevice>,
    pub co_processor: Vec<PciDevice>,
    pub other: Vec<PciDevice>,
}

impl PciDevices {
    pub fn add(&mut self, pci_device: PciDevice) {
        let class = pci_device.header.class_base;
        let sub_class = pci_device.header.class_sub;
        let class_prog_if = pci_device.header.class_prog_if;

        match class {
            0x00 => self.unclassified.push(pci_device),
            0x01 => match sub_class {
                0x00 => self.mass_storage.scsi.push(pci_device),
                0x01 => match class_prog_if {
                    0x00 => self.mass_storage.ide.isa_compatibility.push(pci_device),
                    0x05 => self.mass_storage.ide.pci_native.push(pci_device),
                    _ => self.mass_storage.ide.other.push(pci_device),
                },
                0x02 => self.mass_storage.floppy.push(pci_device),
                0x03 => self.mass_storage.ipi.push(pci_device),
                0x04 => self.mass_storage.raid.push(pci_device),
                0x05 => self.mass_storage.ata.push(pci_device),
                0x06 => match class_prog_if {
                    0x00 => self.mass_storage.sata.vendor_specific.push(pci_device),
                    0x01 => self.mass_storage.sata.ahci.push(pci_device),
                    _ => self.mass_storage.sata.other.push(pci_device),
                },
                0x07 => self.mass_storage.sas.push(pci_device),
                0x08 => self.mass_storage.nvme.push(pci_device),
                0x09 => self.mass_storage.ufs.push(pci_device),
                _ => self.mass_storage.other.push(pci_device),
            },
            0x02 => match sub_class {
                0x00 => self.network.ethernet.push(pci_device),
                0x01 => self.network.token_ring.push(pci_device),
                0x02 => self.network.fddi.push(pci_device),
                0x03 => self.network.atm.push(pci_device),
                0x04 => self.network.isdn.push(pci_device),
                0x05 => self.network.world_fip.push(pci_device),
                0x06 => self.network.picmg.push(pci_device),
                0x07 => self.network.infini_band.push(pci_device),
                0x08 => self.network.fabric.push(pci_device),
                _ => self.network.other.push(pci_device),
            },
            0x03 => match sub_class {
                0x00 => self.display.vga.push(pci_device),
                0x01 => self.display.xga.push(pci_device),
                0x02 => self.display.three_d.push(pci_device),
                _ => self.display.other.push(pci_device),
            },
            0x04 => match sub_class {
                0x00 => self.multimedia.video.push(pci_device),
                0x01 => self.multimedia.audio.push(pci_device),
                0x02 => self.multimedia.telephony.push(pci_device),
                _ => self.multimedia.other.push(pci_device),
            },
            0x05 => match sub_class {
                0x00 => self.memory.ram.push(pci_device),
                0x01 => self.memory.flash.push(pci_device),
                _ => self.memory.other.push(pci_device),
            },
            0x06 => match sub_class {
                0x00 => self.bridge.host.push(pci_device),
                0x01 => self.bridge.isa.push(pci_device),
                0x02 => self.bridge.eisa.push(pci_device),
                0x03 => self.bridge.mca.push(pci_device),
                0x04 => self.bridge.pci_to_pci.push(pci_device),
                0x05 => self.bridge.pcmcia.push(pci_device),
                0x06 => self.bridge.nu_bus.push(pci_device),
                0x07 => self.bridge.card_bus.push(pci_device),
                0x08 => self.bridge.raceway.push(pci_device),
                0x09 => self.bridge.pci_to_pci_non_transparent.push(pci_device),
                0x0A => self.bridge.infini_band.push(pci_device),
                _ => self.bridge.other.push(pci_device),
            },
            0x07 => match sub_class {
                0x00 => self.simple_communication.serial.push(pci_device),
                0x01 => self.simple_communication.parallel.push(pci_device),
                0x02 => self.simple_communication.multiport_serial.push(pci_device),
                0x03 => self.simple_communication.modem.push(pci_device),
                0x04 => self.simple_communication.gpib.push(pci_device),
                0x05 => self.simple_communication.smart_card.push(pci_device),
                _ => self.simple_communication.other.push(pci_device),
            },
            0x08 => match sub_class {
                0x00 => self.base_system_peripheral.pic.push(pci_device),
                0x01 => self.base_system_peripheral.dma.push(pci_device),
                0x02 => self.base_system_peripheral.timer.push(pci_device),
                0x03 => self.base_system_peripheral.rtc.push(pci_device),
                0x04 => self.base_system_peripheral.hot_plug.push(pci_device),
                0x05 => self.base_system_peripheral.sd_host.push(pci_device),
                _ => self.base_system_peripheral.other.push(pci_device),
            },
            0x09 => match sub_class {
                0x00 => self.input.keyboard.push(pci_device),
                0x01 => self.input.digitizer.push(pci_device),
                0x02 => self.input.mouse.push(pci_device),
                0x03 => self.input.scanner.push(pci_device),
                0x04 => self.input.gameport.push(pci_device),
                _ => self.input.other.push(pci_device),
            },
            0x0A => match sub_class {
                0x00 => self.docking.generic.push(pci_device),
                _ => self.docking.other.push(pci_device),
            },
            0x0B => match sub_class {
                0x00 => self.processor.x86.push(pci_device),
                0x01 => self.processor.x86_64.push(pci_device),
                0x10 => self.processor.alpha.push(pci_device),
                0x20 => self.processor.power_pc.push(pci_device),
                0x30 => self.processor.mips.push(pci_device),
                0x40 => self.processor.co_processor.push(pci_device),
                _ => self.processor.other.push(pci_device),
            },
            0x0C => match sub_class {
                0x00 => self.serial_bus.fire_wire.push(pci_device),
                0x01 => self.serial_bus.access_bus.push(pci_device),
                0x02 => self.serial_bus.ssa.push(pci_device),
                0x03 => match class_prog_if {
                    0x00 => self.serial_bus.usb.uhci.push(pci_device),
                    0x10 => self.serial_bus.usb.ohci.push(pci_device),
                    0x20 => self.serial_bus.usb.ehci.push(pci_device),
                    0x30 => self.serial_bus.usb.xhci.push(pci_device),
                    0x80 => self.serial_bus.usb.usb_device.push(pci_device),
                    _ => self.serial_bus.usb.other.push(pci_device),
                },
                0x04 => self.serial_bus.fibre_channel.push(pci_device),
                0x05 => self.serial_bus.sm_bus.push(pci_device),
                0x06 => self.serial_bus.infini_band.push(pci_device),
                0x07 => self.serial_bus.ipmi_smic.push(pci_device),
                0x08 => self.serial_bus.sercos.push(pci_device),
                0x09 => self.serial_bus.can_bus.push(pci_device),
                _ => self.serial_bus.other.push(pci_device),
            },
            0x0D => match sub_class {
                0x00 => self.wireless.bluetooth.push(pci_device),
                0x01 => self.wireless.wifi.push(pci_device),
                _ => self.wireless.other.push(pci_device),
            },
            0x0E => match sub_class {
                0x00 => self.intelligent_io.i2o.push(pci_device),
                _ => self.intelligent_io.other.push(pci_device),
            },
            0x0F => self.satellite.push(pci_device),
            0x10 => match sub_class {
                0x00 => self.encryption.network_crypto.push(pci_device),
                0x10 => self.encryption.entertainment_crypto.push(pci_device),
                _ => self.encryption.other.push(pci_device),
            },
            0x11 => match sub_class {
                0x00 => self.data_acquisition.daq.push(pci_device),
                0x01 => self.data_acquisition.dsp.push(pci_device),
                _ => self.data_acquisition.other.push(pci_device),
            },
            0x12 => match sub_class {
                0x00 => self.processing_accelerator.generic.push(pci_device),
                _ => self.processing_accelerator.other.push(pci_device),
            },
            0x13 => self.non_essential_instrumentation.push(pci_device),
            0x40 => self.co_processor.push(pci_device),
            _ => self.other.push(pci_device),
        }
    }
}
