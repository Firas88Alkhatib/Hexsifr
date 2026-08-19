use core::arch::naked_asm;

#[repr(C)]
#[derive(Default)]
pub struct Context {
    /// Stack pointer after the callee-saved registers have been pushed by
    /// `context_switch`.
    pub stack_pointer: u64,
}

/// Saves the current kernel stack context and resumes `next`.
///
/// # Safety
///
/// Both contexts must refer to valid, live task stacks. `next` must have a
/// complete frame matching the register layout below.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_to(_old: *mut Context, _next: *const Context) {
    naked_asm!(
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov [rdi], rsp",
        "mov rsp, [rsi]",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "ret",
    );
}
