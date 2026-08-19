use alloc::boxed::Box;

use crate::scheduler::Context;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct TaskId(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Priority {
    Low,
    Normal,
    High,
}

impl Priority {
    pub(crate) const fn time_slice_ticks(self) -> u32 {
        match self {
            Self::High => 8,
            Self::Normal => 4,
            Self::Low => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskState {
    Ready,
    Running,
}

pub(crate) struct Task {
    pub(crate) id: TaskId,
    pub(crate) context: Context,
    pub(crate) _stack: Box<[u8]>,
    pub(crate) entry: fn(),
    pub(crate) priority: Priority,
    pub(crate) remaining_ticks: u32,
    pub(crate) state: TaskState,
}
