pub(crate) mod frame_allocator;
pub(crate) mod heap_allocator;
pub(crate) mod mapper;
pub(crate) mod physical;

use bootloader_api::info::{MemoryRegion, MemoryRegionKind, MemoryRegions};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::memory::{frame_allocator::init_frame_allocator, heap_allocator::init_heap};

static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);

pub(crate) fn memory_init(memory_regions: &mut MemoryRegions) {
    info!("Initializing frame allocator");
    init_frame_allocator(memory_regions);
    init_heap();
}

pub(crate) fn set_physical_memory_offset(offset: u64) {
    PHYSICAL_MEMORY_OFFSET.store(offset, Ordering::Relaxed);
}

pub(crate) fn physical_memory_offset() -> u64 {
    PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed)
}

pub(crate) fn phys_to_virt<T>(phys: u64) -> *const T {
    (physical_memory_offset() + phys) as *const T
}

pub(crate) fn phys_to_virt_mut<T>(phys: u64) -> *mut T {
    (physical_memory_offset() + phys) as *mut T
}
/// Converts a physical address to a mutable reference.
/// # Safety
/// The physical address must be valid and correctly aligned.
pub fn phys_to_virt_ref_mut<T>(phys: u64) -> &'static mut T {
    unsafe { &mut *(phys_to_virt_mut::<T>(phys)) }
}

// pub(crate) fn phys_to_virt_unaligned<T>(phys: u64) -> T {
//     unsafe { phys_to_virt::<T>(phys).read_unaligned() }
// }

pub trait MemoryRegionsExt {
    fn usable_regions(&self) -> impl Iterator<Item = &MemoryRegion>;

    fn usable_regions_mut(&mut self) -> impl Iterator<Item = &mut MemoryRegion>;

    fn usable_regions_boudry(&self) -> (u64, u64);
}

impl MemoryRegionsExt for MemoryRegions {
    fn usable_regions(&self) -> impl Iterator<Item = &MemoryRegion> {
        self.iter().filter(|r| r.kind == MemoryRegionKind::Usable)
    }

    fn usable_regions_mut(&mut self) -> impl Iterator<Item = &mut MemoryRegion> {
        self.iter_mut().filter(|r| r.kind == MemoryRegionKind::Usable)
    }

    fn usable_regions_boudry(&self) -> (u64, u64) {
        let min_start = self.usable_regions().map(|r| r.start).min().unwrap_or(0);
        let max_end = self.usable_regions().map(|r| r.end).max().unwrap_or(0);
        (min_start, max_end)
    }
}
