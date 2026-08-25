use core::{arch::naked_asm, ops::Sub};

use alloc::boxed::Box;

const KERNEL_STACK_SIZE: usize = 64 * 1024;
const CONTEXT_SAVED_REGISTERS: usize = 6;
const INITIAL_CONTEXT_WORDS: usize = CONTEXT_SAVED_REGISTERS + 2;
const FRAME_SIZE: usize = INITIAL_CONTEXT_WORDS * size_of::<usize>();

#[repr(C)]
pub struct TaskContext {
    /// Stack pointer after the callee-saved registers have been pushed by
    /// `context_switch`. this needs to be always at the first offset
    pub stack_pointer: u64,
    pub stack: Box<[u8; KERNEL_STACK_SIZE]>,
}

impl TaskContext {
    pub fn new(entry: extern "C" fn()) -> Self {
        let mut stack = Box::new([0_u8; KERNEL_STACK_SIZE]);
        let aligned_stack_end = unsafe { stack.as_mut_ptr().add(stack.len()) } as usize & !0xF;
        let stack_pointer = aligned_stack_end.sub(FRAME_SIZE) as u64;
        let frame = stack_pointer as *mut usize;
        unsafe { frame.add(CONTEXT_SAVED_REGISTERS).write(entry as *const () as usize) }

        Self { stack_pointer, stack }
    }
    pub fn reset(&mut self, entry: extern "C" fn()) {
        let frame = self.stack_pointer as *mut usize;
        unsafe { frame.add(CONTEXT_SAVED_REGISTERS).write(entry as *const () as usize) };
    }
}

/// Saves the current kernel stack context and resumes `next`.
///
/// # Safety
///
/// Both contexts must refer to valid, live task stacks. `next` must have a
/// complete frame matching the register layout below.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_to(_old: *mut TaskContext, _next: *const TaskContext) {
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
