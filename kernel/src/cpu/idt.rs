use spin::Once;
use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

use super::{
    gdt::ist,
    halt_loop,
    lapic::read_lapic_error_status_register,
    per_cpu::{get_cpu_id, get_cpu_info},
};

pub const TIMER_VECTOR: u8 = 32;
pub const APIC_ERROR_VECTOR: u8 = 33;
pub const XHCI_MSIX_VECTOR: u8 = 34;

pub const SPURIOUS_VECTOR: u8 = 255;

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn eoi() {
    unsafe { get_cpu_info().lapic.end_of_interrupt() };
}

pub fn get_idt() -> &'static InterruptDescriptorTable {
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        // 0 = Divide Error
        // triggered when the CPU executes a division instruction with divisor zero or
        // when the quotient ( the result of a division ) cannot be represented in the destination register.
        idt.divide_error.set_handler_fn(divide_error_handler);
        // 1 = Debug
        // triggered by CPU debug features such as single-stepping, hardware breakpoints,
        // or watchpoints used for low-level debugging.
        idt.debug.set_handler_fn(debug_handler);
        // 2 = Non-Maskable Interrupt (NMI):
        // A high-priority hardware interrupt that cannot be disabled or ignored by the CPU;
        // used for critical hardware errors (e.g. memory corruption or watchdog events).
        unsafe { idt.non_maskable_interrupt.set_handler_fn(nmi_handler).set_stack_index(ist::NMI as u16) };
        // 3 = Breakpoint
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        // 4 = Overflow:
        // Occurs when the result of an arithmetic operation is too large to fit in
        // the destination register (signed arithmetic overflow exception).
        // example :
        // let x: i8 = 100; let y = x + 50; // overflow in i8 max value 127
        idt.overflow.set_handler_fn(overflow_handler);
        // 5 = Bound Range Exceeded
        // legacy exception triggered by the obsolete BOUND instruction when
        // an index is outside predefined bounds.
        idt.bound_range_exceeded.set_handler_fn(bound_range_handler);
        // 6 = Invalid Opcode
        // CPU attempted to execute an undefined or unsupported instruction.
        // Indicates corrupted code, invalid memory execution, or unsupported CPU features.
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        // 7 = Device Not Available
        // triggered when executing FPU(Floating-point unit)/SIMD instructions while the CPU has not enabled or
        // loaded floating-point state (used for lazy FPU context switching).
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        // 8 = Double Fault
        // SAFETY: DOUBLE_FAULT_IST_INDEX (0) is a valid IST index pointing to
        // the dedicated stack allocated in gdt.rs. Using IST prevents triple
        // faults when the double fault occurs due to kernel stack overflow.
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(ist::DOUBLE_FAULT as u16);
        }
        // 9 = Coprocessor Segment Overrun (legacy, rarely used), ignored in rust-x86_64 crate

        // 10 = Invalid TSS
        // triggered when the CPU attempts to use a Task State Segment for stack
        // or privilege switching, but the TSS or its descriptor is invalid or misconfigured.
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        // 11 = Segment Not Present
        // triggered when the CPU attempts to use a segment selector whose descriptor is
        // invalid or marked as not present in the GDT/LDT.
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        // 12 = Stack Segment Fault
        // triggered when the CPU attempts to use or switch to a stack segment (SS) that is invalid,
        // not present, or violates protection rules during stack operations or privilege transitions.
        // This is a low-level kernel integrity error, not a normal application bug.
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        // 13 = General Protection Fault
        // triggered when the CPU detects a protection violation that does not fall into a more
        // specific exception category (e.g., invalid segment usage, privilege violation,
        //or descriptor table error).
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        // 14 = Page Fault
        // triggered when the CPU cannot translate a virtual address or when memory access violates
        // paging permissions (read/write/execute or user/supervisor rules).
        idt.page_fault.set_handler_fn(page_fault_handler);

        // 15 = Reserved and should not assign handler to it, as per Intel manuals.
        // If this vector is triggered, it indicates a serious CPU error or misconfiguration.

        // 16 = x87 Floating-Point Exception
        // triggered when the legacy x87 FPU detects an invalid arithmetic operation such as
        // division by zero, overflow, or invalid stack state.
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        // 17 = Alignment Check
        // mostly unused in modern operating systems.
        // triggered when the CPU detects misaligned memory access while alignment checking is
        // enabled (AC flag set), typically used for debugging or strict memory alignment enforcement.
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        // 18 = Machine Check (fatal hardware error)
        // one of the most serious CPU exceptions
        // triggered when the CPU detects a fatal or serious hardware error
        // (CPU, cache, memory, or bus corruption) reported via Machine Check Architecture.
        unsafe { idt.machine_check.set_handler_fn(machine_check_handler).set_stack_index(ist::MACHINE_CHECK as u16) };
        // 19 = SIMD Floating-Point Exception (SSE/AVX)
        // triggered when SSE/AVX vector floating-point operations produce errors such as
        // divide-by-zero, overflow, or invalid arithmetic under enabled exception masks.
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        // 20 = Virtualization Exception
        // triggered by hardware virtualization (VT-x/AMD-V) events such as VM exits or
        // invalid virtualization control state requiring hypervisor handling.
        idt.virtualization.set_handler_fn(virtualization_handler);

        // 21–29 = Reserved
        // These vectors are reserved by Intel and should not be used. If triggered, they indicate a serious CPU error or
        // misconfiguration.

        // 30 = Security Exception
        // triggered when CPU-enforced security mechanisms (SMEP, SMAP, CET, etc.) detect
        // a violation of hardware-level protection rules.
        idt.security_exception.set_handler_fn(security_exception_handler);

        // 31 = Reserved and should not assign handler to it, as per Intel manuals.

        // APIC / IRQs start at vector 32.

        // 32 Lapic timer
        idt[TIMER_VECTOR].set_handler_fn(timer_interrupt_handler);
        // 33 APIC Error vector
        idt[APIC_ERROR_VECTOR].set_handler_fn(apic_error_interrupt_handler);
        // 34 XHCI
        idt[XHCI_MSIX_VECTOR].set_handler_fn(xhci_msix_handler);
        // 255 sporious vector
        idt[SPURIOUS_VECTOR].set_handler_fn(spurious_interrupt_handler);
        return idt;
    })
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DIVIDE ERROR");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DEBUG");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn nmi_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: NON-MASKABLE INTERRUPT");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BREAKPOINT");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: OVERFLOW");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn bound_range_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BOUND RANGE EXCEEDED");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: INVALID OPCODE");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DEVICE NOT AVAILABLE (FPU/SIMD)");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    error!("EXCEPTION: DOUBLE FAULT");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: INVALID TSS");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: SEGMENT NOT PRESENT");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: STACK SEGMENT FAULT");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: GENERAL PROTECTION FAULT");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error Code: {:#x}", error_code);
    error!("{:#?}", stack_frame);
    halt_loop();
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    error!("EXCEPTION: PAGE FAULT");
    error!("CPU ID: {}", get_cpu_id());
    error!("Accessed Address: {:?}", Cr2::read());
    error!("Error Code: {:?}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: x87 FLOATING POINT ERROR");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: ALIGNMENT CHECK");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    error!("EXCEPTION: MACHINE CHECK (HARDWARE FAILURE)");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: SIMD FLOATING POINT ERROR");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: VIRTUALIZATION");
    error!("CPU ID: {}", get_cpu_id());
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: SECURITY EXCEPTION");
    error!("CPU ID: {}", get_cpu_id());
    error!("Error code: {:#x}", error_code);
    error!("{:#?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Acknowledge the interrupt first so the LAPIC can deliver the next one.
    eoi();
    get_cpu_info().scheduler.on_timer_tick();
}

extern "x86-interrupt" fn apic_error_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let esr = read_lapic_error_status_register();
    error!("APIC error: ESR = {:#x}", esr);
    error!("CPU ID: {}", get_cpu_id());
    eoi();
}

// A spurious interrupt is an interrupt that gets signaled even though no real
// hardware interrupt happened.This can happen due to electrical noise,
// race conditions in the interrupt controller, interrupt being acknowledged at the wrong time,
// hardware quirks in older PIC systems etc..
// We must handle it, otherwise we might get double faults, or triple faults if the spurious
// interrupt happens while handling another interrupt.
extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
    info!("[lapic] spurious interrupt");
}

extern "x86-interrupt" fn xhci_msix_handler(_stack: InterruptStackFrame) {
    // Process event ring events...
    crate::drivers::usb::xhci::handle_interrupt();
    eoi();
}
