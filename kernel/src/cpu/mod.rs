use x86_64::instructions::interrupts;

pub(crate) mod ap;
pub(crate) mod gdt;
pub(crate) mod idt;
pub(crate) mod lapic;
pub(crate) mod per_cpu;

pub(crate) fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
pub fn cpu_init() {
    per_cpu::init_bsp_per_cpu();
    interrupts::enable();
}
