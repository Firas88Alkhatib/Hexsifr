use super::context::TaskContext;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WakeAt(pub u128);

pub(crate) struct Task {
    pub(crate) id: TaskId,
    pub(crate) context: TaskContext,
    pub(crate) entry: fn(),
    pub(crate) priority: Priority,
    pub(crate) remaining_ticks: u32,
    pub wake_at: Option<WakeAt>,
}
