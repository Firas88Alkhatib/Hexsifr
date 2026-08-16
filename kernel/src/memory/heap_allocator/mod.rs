pub mod slap_bool;

use crate::memory::heap_allocator::slap_bool::get_slap_bool;
use crate::memory::physical::reserve_memory;
use crate::memory::{phys_to_virt, physical_memory_offset};
use bootloader_api::info::MemoryRegions;
use buddy_slab_allocator::eii::{slab_pool_impl, virt_to_phys_impl};
use buddy_slab_allocator::{GlobalAllocator, SlabPoolTrait};

const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

#[virt_to_phys_impl]
fn virt_to_phys(vaddr: usize) -> usize {
    let offset = physical_memory_offset() as usize;
    vaddr.checked_sub(offset).expect("virt_to_phys: address below physical offset")
}

#[slab_pool_impl]
fn slab_pool() -> &'static dyn SlabPoolTrait {
    get_slap_bool()
}

pub(crate) fn init_heap(memory_regions: &mut MemoryRegions) {
    let heap_phys = reserve_memory(memory_regions, HEAP_SIZE).expect("Failed to reserve memory for heap");
    let heap_virt = phys_to_virt::<u8>(heap_phys) as *mut u8;
    let heap_slice = unsafe { core::slice::from_raw_parts_mut(heap_virt, HEAP_SIZE) };

    unsafe {
        ALLOCATOR.init(heap_slice).expect("Failed to init buddy allocator");
    }
}
