use core::sync::atomic::{AtomicU64, Ordering};
pub (crate) mod physical;
pub(crate) mod frame_allocator;


static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);

pub(crate) fn set_physical_memory_offset(offset: u64) {
    PHYSICAL_MEMORY_OFFSET.store(offset, Ordering::Relaxed);
}

pub(crate) fn physical_memory_offset() -> u64 {
    PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed)
}

pub(crate) fn phys_to_virt<T>(phys: u64) -> *const T {
    (physical_memory_offset() + phys) as *const T
}

pub(crate) fn phys_to_virt_unaligned<T>(phys: u64) -> T {
    unsafe { phys_to_virt::<T>(phys).read_unaligned() }
}
