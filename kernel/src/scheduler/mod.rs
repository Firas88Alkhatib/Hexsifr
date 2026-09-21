use core::time::Duration;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    vec::Vec,
};

pub mod context;
pub mod task;

use context::{TaskContext, switch_to};

use task::{Priority, Task, TaskId, WakeAt};
use x86_64::instructions::interrupts::{self, without_interrupts};

use crate::{
    cpu::{halt_loop, per_cpu::get_cpu_info},
    time::current_time_ns,
};

pub struct Scheduler {
    pub run_queue: VecDeque<Box<Task>>,
    pub current_task: Option<Box<Task>>,
    pub sleep_queue: BTreeMap<WakeAt, Vec<Box<Task>>>,
    pub next_task_id: usize,
    pub idle_context: TaskContext,
    pub exit_context: TaskContext,
    started: bool,
}

/**
 * Weighted round-robin scheduler.
 *
 * Higher-priority tasks receive longer time slices, giving them a larger
 * share of CPU time while preserving round-robin fairness among runnable tasks.
 * Scheduling is preemptive based on time-slice expiration.
 */
impl Scheduler {
    pub fn new() -> Self {
        let idle_context = TaskContext::new(idle_loop);
        let exit_context = TaskContext::new(task_exit_trampoline);
        Self {
            next_task_id: 0,
            run_queue: VecDeque::new(),
            sleep_queue: BTreeMap::new(),
            current_task: None,
            idle_context,
            exit_context,
            started: false,
        }
    }
    pub fn start(&mut self) {
        self.started = true;
    }
    pub fn add_task(&mut self, entry: fn(), priority: Priority) {
        without_interrupts(|| {
            let id = TaskId(self.next_task_id);
            self.next_task_id = self.next_task_id.checked_add(1).expect("task ID space exhausted");

            let context = TaskContext::new(task_bootstrap);
            let new_task =
                Task { id, entry, priority, remaining_ticks: priority.time_slice_ticks(), context, wake_at: None };
            self.run_queue.push_back(Box::new(new_task));
        })
    }
    pub fn sleep_current_task(&mut self, duration: Duration) {
        without_interrupts(|| {
            let mut current_task = self.current_task.take().expect("No current task to sleep");
            let current_context = &raw mut current_task.context;

            let wake_at = WakeAt(current_time_ns() + duration.as_nanos());
            current_task.wake_at = Some(wake_at);

            self.sleep_queue.entry(wake_at).or_default().push(current_task);

            let next_context = self.take_next_task().unwrap_or(&raw mut self.idle_context);

            unsafe { switch_to(current_context, next_context) }
        });
    }

    fn wake_sleeping_tasks(&mut self) {
        let now = current_time_ns();
        while let Some(entry) = self.sleep_queue.first_entry() {
            if entry.key().0 > now {
                break;
            }

            let (_, tasks) = entry.remove_entry();

            for mut task in tasks {
                task.wake_at = None;
                self.run_queue.push_back(task);
            }
        }
    }
    pub fn switch_to_exit_context(&mut self) {
        interrupts::without_interrupts(|| {
            let current_task = self.current_task.as_mut().expect("task finished without a current task");
            let old_context = &raw mut current_task.context;

            self.exit_context.reset(task_exit_trampoline);
            let exit_context = &raw mut self.exit_context;

            unsafe { switch_to(old_context, exit_context) };
        })
    }

    fn take_next_task(&mut self) -> Option<*const TaskContext> {
        let next_task = self.run_queue.pop_front()?;

        let next_context = &raw const next_task.context;
        self.current_task = Some(next_task);

        Some(next_context)
    }

    pub fn on_timer_tick(&mut self) {
        without_interrupts(|| {
            if !self.started {
                return;
            }
            self.wake_sleeping_tasks();

            if let Some(current_task) = &mut self.current_task {
                if current_task.remaining_ticks > 0 {
                    current_task.remaining_ticks -= 1;
                    return;
                }

                // current task expired, reset its timer
                current_task.remaining_ticks = current_task.priority.time_slice_ticks();

                // switch to next task
                if let Some(mut next_task) = self.run_queue.pop_front() {
                    let next_context = &raw mut next_task.context;

                    let mut current_task = self
                        .current_task
                        .replace(next_task)
                        .expect("Trying to replace current task with next task without having current task");

                    let current_context = &raw mut current_task.context;

                    self.run_queue.push_back(current_task);

                    unsafe { switch_to(current_context, next_context) }
                }
                // no next task, continue the current one
                return;
            };

            // no current task, meaning we are on idle loop
            // check for new task to switch to
            if let Some(next_context) = self.take_next_task() {
                let current_context = &raw mut self.idle_context;
                unsafe { switch_to(current_context, next_context) }
            }

            return;
        });
    }
}

extern "C" fn idle_loop() {
    halt_loop()
}

extern "C" fn task_bootstrap() {
    let entry = {
        let scheduler = &get_cpu_info().scheduler;
        scheduler.current_task.as_ref().expect("task started without a current task").entry
    };
    interrupts::enable();
    entry();
    get_cpu_info().scheduler.switch_to_exit_context()
}

extern "C" fn task_exit_trampoline() {
    let scheduler = &mut get_cpu_info().scheduler;
    let current_task = scheduler.current_task.take().expect("scheduler exit without a current task");
    drop(current_task);

    let exit_context = &raw mut scheduler.exit_context;

    let next_context = scheduler.take_next_task().unwrap_or(&raw mut scheduler.idle_context);

    unsafe { switch_to(exit_context, next_context) }
}
