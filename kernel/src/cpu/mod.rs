pub(crate) mod gdt;
pub(crate) mod idt;
pub(crate) mod lapic;

pub(crate) fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
