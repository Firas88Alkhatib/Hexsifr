//! A small per-CPU, priority-aware kernel-thread scheduler.

use alloc::{boxed::Box, collections::VecDeque};
use x86_64::instructions::interrupts;

mod context;
mod task;

use crate::cpu::{halt_loop, per_cpu::get_cpu_info};
use crate::scheduler::task::{Task, TaskId, TaskState};

pub(crate) use context::Context;
pub(crate) use task::Priority;

const KERNEL_STACK_SIZE: usize = 64 * 1024;
const CONTEXT_SAVED_REGISTERS: usize = 6;
const INITIAL_CONTEXT_WORDS: usize = CONTEXT_SAVED_REGISTERS + 2;

struct ExitStack {
    context: Context,
    stack: Box<[u8]>,
}

#[derive(Default)]
pub(crate) struct PerCpuScheduler {
    run_queue: VecDeque<Box<Task>>,
    current: Option<Box<Task>>,
    next_task_id: usize,
    boot_context: Context,
    exit_stack: Option<ExitStack>,
}

impl PerCpuScheduler {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn take_next_task(&mut self) -> Option<Box<Task>> {
        let priority = self.run_queue.iter().map(|task| task.priority).max()?;
        let index = self.run_queue.iter().position(|task| task.priority == priority).expect("ready task missing from run queue");
        self.run_queue.remove(index)
    }
}

/// Prepares the current CPU's scheduler after heap initialization.
pub fn init() {
    interrupts::without_interrupts(|| {
        let mut scheduler = get_cpu_info().scheduler.lock();
        assert!(scheduler.exit_stack.is_none(), "scheduler initialized twice");

        let mut stack = Box::new([0_u8; KERNEL_STACK_SIZE]);
        let context = initial_context(&mut *stack, task_exit_trampoline);
        scheduler.exit_stack = Some(ExitStack { context, stack });
    });
}

/// Adds a task to the calling CPU's run queue.
pub fn spawn(entry: fn(), priority: Priority) {
    let mut stack = Box::new([0_u8; KERNEL_STACK_SIZE]);
    let context = initial_context(&mut *stack, task_bootstrap);

    interrupts::without_interrupts(|| {
        let mut scheduler = get_cpu_info().scheduler.lock();
        assert!(scheduler.current.is_none(), "spawning after scheduler start is not supported yet");

        let id = TaskId(scheduler.next_task_id);
        scheduler.next_task_id = scheduler.next_task_id.checked_add(1).expect("task ID space exhausted");
        scheduler.run_queue.push_back(Box::new(Task {
            id,
            context,
            _stack: stack,
            entry,
            priority,
            remaining_ticks: priority.time_slice_ticks(),
            state: TaskState::Ready,
        }));
    });
}

/// Starts the first task on the current CPU.
pub fn start() -> ! {
    interrupts::without_interrupts(|| {
        let switch = {
            // scheduler here is borrowed mutable , it needs to be in a scope
            // in order to unlock it before we call context switch
            let mut scheduler = get_cpu_info().scheduler.lock();
            let mut next = scheduler.take_next_task().expect("scheduler started without tasks");
            next.state = TaskState::Running;
            next.remaining_ticks = next.priority.time_slice_ticks();
            let next_context = &next.context as *const Context;
            scheduler.current = Some(next);
            (&mut scheduler.boot_context as *mut Context, next_context)
        };

        unsafe { context::switch_to(switch.0, switch.1) };
        unreachable!("the boot context cannot be scheduled")
    })
}

/// Called by the local APIC timer interrupt.
pub fn on_timer_tick() {
    let switch = {
        // scheduler here is borrowed mutable , it needs to be in a scope
        // in order to unlock it before we call context switch
        let mut scheduler = get_cpu_info().scheduler.lock();
        let current_id = match scheduler.current.as_ref() {
            Some(task) => task.id,
            None => return,
        };
        let old_context = {
            let current = scheduler.current.as_mut().expect("current task disappeared");
            if current.remaining_ticks > 1 {
                current.remaining_ticks -= 1;
                return;
            }
            current.state = TaskState::Ready;
            &mut current.context as *mut Context
        };

        let current = scheduler.current.take().expect("current task disappeared");
        scheduler.run_queue.push_back(current);
        let mut next = scheduler.take_next_task().expect("current task was not requeued");
        next.state = TaskState::Running;
        next.remaining_ticks = next.priority.time_slice_ticks();

        if next.id == current_id {
            scheduler.current = Some(next);
            return;
        }

        let next_context = &next.context as *const Context;
        scheduler.current = Some(next);
        (old_context, next_context)
    };

    unsafe { context::switch_to(switch.0, switch.1) };
}

extern "C" fn task_bootstrap() -> ! {
    let entry = {
        let scheduler = get_cpu_info().scheduler.lock();
        scheduler.current.as_ref().expect("task started without a current task").entry
    };
    interrupts::enable();
    entry();
    finish_current_task()
}

fn finish_current_task() -> ! {
    interrupts::without_interrupts(|| {
        let switch = {
            let mut scheduler = get_cpu_info().scheduler.lock();
            let old_context = &mut scheduler.current.as_mut().expect("task finished without a current task").context as *mut Context;

            let exit_stack = scheduler.exit_stack.as_mut().expect("scheduler is not initialized");
            exit_stack.context = initial_context(&mut *exit_stack.stack, task_exit_trampoline);

            let exit_context = &scheduler.exit_stack.as_ref().expect("scheduler is not initialized").context as *const Context;
            (old_context, exit_context)
        };

        unsafe { context::switch_to(switch.0, switch.1) };
        unreachable!("a finished task was scheduled")
    })
}

extern "C" fn task_exit_trampoline() -> ! {
    let switch = {
        let mut scheduler = get_cpu_info().scheduler.lock();
        drop(scheduler.current.take().expect("scheduler exit without a current task"));

        let mut next = scheduler.take_next_task().unwrap_or_else(|| halt_loop());
        next.state = TaskState::Running;
        next.remaining_ticks = next.priority.time_slice_ticks();
        let next_context = &next.context as *const Context;
        scheduler.current = Some(next);

        let exit_context = &mut scheduler.exit_stack.as_mut().expect("scheduler is not initialized").context as *mut Context;
        (exit_context, next_context)
    };

    unsafe { context::switch_to(switch.0, switch.1) };
    unreachable!("scheduler exit stack was resumed")
}

fn initial_context(stack: &mut [u8], entry: extern "C" fn() -> !) -> Context {
    let top = unsafe { stack.as_mut_ptr().add(stack.len()) } as usize & !0xF;
    let pointer = top - INITIAL_CONTEXT_WORDS * size_of::<usize>();
    unsafe {
        let frame = pointer as *mut usize;
        frame.write_bytes(0, CONTEXT_SAVED_REGISTERS);
        frame.add(CONTEXT_SAVED_REGISTERS).write(entry as *const () as usize);
    }
    Context { stack_pointer: pointer as u64 }
}
