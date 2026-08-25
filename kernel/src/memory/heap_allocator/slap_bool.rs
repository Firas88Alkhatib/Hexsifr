use core::mem::size_of;

use bootloader_api::info::MemoryRegions;
use buddy_slab_allocator::{PerCpuSlab, SlabPoolTrait, SlabTrait};

use spin::Once;
use x86_64::{
    align_up,
    structures::paging::{PageSize, Size4KiB},
};

use crate::{
    acpi::get_cpu_count,
    cpu::per_cpu::get_cpu_id,
    memory::{phys_to_virt, physical::reserve_memory},
};

const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
static SLAB_POOL: Once<DynamicSlabPool<{ PAGE_SIZE }>> = Once::new();
pub struct DynamicSlabPool<const PAGE_SIZE: usize> {
    pools: &'static [PerCpuSlab<PAGE_SIZE>],
}

// Mark as `Sync` – required by the trait.
unsafe impl<const PAGE_SIZE: usize> Sync for DynamicSlabPool<PAGE_SIZE> {}
impl<const PAGE_SIZE: usize> SlabPoolTrait for DynamicSlabPool<PAGE_SIZE> {
    fn current_slab(&self) -> &dyn SlabTrait {
        let cpu_id = get_cpu_id();
        &self.pools[cpu_id]
    }

    fn owner_slab(&self, cpu_idx: usize) -> &dyn SlabTrait {
        &self.pools[cpu_idx]
    }
}
pub fn init_slab_pool(memory_regions: &mut MemoryRegions) {
    SLAB_POOL.call_once(|| {
        let cpu_count = get_cpu_count();
        let slab_size = size_of::<PerCpuSlab<PAGE_SIZE>>();
        let total_size = slab_size * cpu_count;
        let total_bytes = align_up(total_size as u64, PAGE_SIZE as u64);

        let slab_phys = reserve_memory(memory_regions, total_bytes as usize).expect("Failed to reserve memory for slab pool");

        let slab_virt = phys_to_virt::<PerCpuSlab<PAGE_SIZE>>(slab_phys) as *mut PerCpuSlab<PAGE_SIZE>;
        let slab_slice = unsafe { core::slice::from_raw_parts_mut(slab_virt, cpu_count) };

        for (i, slab) in slab_slice.iter_mut().enumerate() {
            *slab = PerCpuSlab::new(i as u16);
        }

        let pool = DynamicSlabPool { pools: slab_slice };
        pool
    });
}

pub fn get_slap_bool() -> &'static dyn SlabPoolTrait {
    SLAB_POOL.get().expect("Slab pool not initialised")
}
