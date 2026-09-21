use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::ops::Deref;

use alloc::borrow::ToOwned;
use x86_64::VirtAddr;

use crate::drivers::usb::xhci::dcbaa::Dcbaa;
use crate::drivers::usb::xhci::rings::Ring;
use crate::drivers::usb::xhci::rings::command::CommandRing;

#[repr(transparent)]
pub struct Reg<T> {
    value: UnsafeCell<T>,
}

impl<T> Clone for Reg<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl<T: Copy> Reg<T> {
    pub fn read(&self) -> T {
        unsafe { self.value.get().read_volatile() }
    }

    pub fn write(&self, value: T) {
        unsafe { self.value.get().write_volatile(value) }
    }

    pub fn modify<F: FnOnce(T) -> T>(&self, f: F) {
        self.write(f(self.read()));
    }
}

#[derive(Clone)]
pub struct Mmio<T> {
    virt: VirtAddr,
    _marker: PhantomData<T>,
}

impl<T> Mmio<T> {
    pub const fn new(addr: VirtAddr) -> Self {
        Self { virt: addr, _marker: PhantomData }
    }
    pub fn addr(&self) -> VirtAddr {
        self.virt
    }
}

impl<T> Deref for Mmio<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.virt.as_mut_ptr::<T>() }
    }
}

/*
 xHCI MMIO BAR
0x00
│
├── Capability Registers
│   ├── 0x00 CAPLENGTH + HCIVERSION
│   ├── 0x04 HCSPARAMS1
│   ├── 0x08 HCSPARAMS2
│   ├── 0x0C HCSPARAMS3
│   ├── 0x10 HCCPARAMS1
│   ├── ...
│   └── ...
│
│   CAPLENGTH ───────────────┐
│                            │
├── Operational Registers ◄──┘
│   ├── 0x00 USBCMD
│   ├── 0x04 USBSTS
│   ├── 0x08 PAGESIZE
│   ├── 0x10 DNCTRL
│   ├── 0x14 CRCR low
│   ├── 0x18 CRCR high
│   ├── ...
│   └── PORTSC registers
│       ├── 0x400 PORTSC1
│       ├── 0x404 PORTSC2
│       ├── 0x408 PORTSC3
│       └── ...
│
├── Runtime Registers
│   ├── MFINDEX
│   ├── Interrupter 0
│   │   ├── IMAN
│   │   ├── IMOD
│   │   ├── ERSTSZ
│   │   ├── ERSTBA
│   │   └── ERDP
│   └── Interrupter 1...
│
├── Doorbell Registers
│   ├── Doorbell 0
│   ├── Doorbell 1
│   ├── Doorbell 2
│   └── ...
│
└── Extended Capability Registers
    └── (if present)
 */

#[repr(C)]
pub struct CapabilityRegs {
    pub cap_length: Reg<u8>,
    _reserved: Reg<u8>,
    pub hci_version: Reg<u16>,
    pub hcs_params1: Reg<u32>,
    pub hcs_params2: Reg<u32>,
    pub hcs_params3: Reg<u32>,
    pub hcc_params1: Reg<u32>,
    pub db_off: Reg<u32>,
    pub rts_off: Reg<u32>,
    pub hcc_params2: Reg<u32>,
}
impl CapabilityRegs {
    pub fn get_max_ports(&self) -> u8 {
        let hcs1 = self.hcs_params1.read();
        (hcs1 >> 24) as u8
    }

    pub fn get_max_slots(&self) -> u8 {
        let hcs1 = self.hcs_params1.read();
        hcs1 as u8
    }
    pub fn get_max_interrupters(&self) -> u16 {
        let hcs1 = self.hcs_params1.read();
        ((hcs1 >> 8) & 0x7ff) as u16
    }
    pub fn get_max_scratchpads(&self) -> u32 {
        let hcs2 = self.hcs_params2.read();
        let scratchpad_hi = (hcs2 >> 21) & 0x1f;
        let scratchpad_lo = (hcs2 >> 27) & 0x1f;
        (scratchpad_hi << 5) | scratchpad_lo
    }
    pub fn get_extended_capabilities_offset(&self) -> u32 {
        ((self.hcc_params1.read() >> 16) as u16 as u32) * 4
    }
    pub fn get_context_size(&self) -> usize {
        const CSZ: u32 = 1 << 2;
        if self.hcc_params1.read() & CSZ != 0 { 64 } else { 32 }
    }
}

#[repr(C)]
pub struct OperationalRegs {
    pub usbcmd: Reg<u32>,
    pub usbsts: Reg<u32>,
    pub pagesize: Reg<u32>,
    _reserved1: [Reg<u32>; 2],
    pub dnctrl: Reg<u32>,
    pub crcr: Reg<u64>,
    _reserved2: [Reg<u32>; 4],

    pub dcbaap: Reg<u64>,
    pub config: Reg<u32>,
    _reserved3: [Reg<u32>; 241],
}
impl OperationalRegs {
    pub fn is_halted(&self) -> bool {
        const HALTED_MASK: u32 = 1 << 0;
        self.usbsts.read() & HALTED_MASK == HALTED_MASK
    }
    pub fn is_ready(&self) -> bool {
        const NOT_READY_MASK: u32 = 1 << 11;
        self.usbsts.read() & NOT_READY_MASK == 0
    }
    pub fn reset(&self) {
        const HCRST: u32 = 1 << 1; // Host Controller Reset
        self.usbcmd.modify(|v| v | HCRST);

        while self.usbcmd.read() & HCRST != 0 {
            spin_loop();
        }
        while !self.is_ready() {
            spin_loop();
        }
    }

    pub fn stop(&self) {
        const STOP_MASK: u32 = !(1 << 0);
        self.usbcmd.write(self.usbcmd.read() & STOP_MASK);
        while self.is_halted() && !self.is_ready() {
            spin_loop();
        }
    }
    pub fn start_and_enable(&self) {
        const START_MASK: u32 = 1 << 0;
        const ENABLE_INTERRUPTS_MASK: u32 = 1 << 2;
        const START_AND_ENABLE: u32 = START_MASK | ENABLE_INTERRUPTS_MASK;

        self.usbcmd.modify(|value| value | START_AND_ENABLE);
        while self.is_halted() || !self.is_ready() {
            spin_loop();
        }
    }

    pub fn enable_all_notifications(&self) {
        self.dnctrl.write(0xffff);
    }

    pub fn config_max_slots(&self, max_slots: u8) {
        self.config.write(max_slots as u32);
    }
    pub fn set_command_ring(&self, command_ring: &Ring<CommandRing>) {
        self.crcr.write(command_ring.phys_region.phys.as_u64() | command_ring.cycle as u64);
    }
    pub fn set_dcbaa(&self, dcbaa: &Dcbaa) {
        self.dcbaap.write(dcbaa.phys_region.phys.as_u64());

        info!("DCBAAP = {:#x}", self.dcbaap.read());
    }

    pub fn eoi(&self) {
        const USBSTS_EINT: u32 = 1 << 3;
        self.usbsts.write(USBSTS_EINT);
    }
}

#[repr(C)]
pub struct RuntimeRegs {
    pub mfindex: Reg<u32>, // microframe index used for scheduling
    _reserved: [Reg<u32>; 7],
}
