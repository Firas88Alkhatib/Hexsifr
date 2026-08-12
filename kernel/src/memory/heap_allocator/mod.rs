use crate::cpu;
use crate::memory::mapper::active_page_table_mapper;
use crate::memory::physical_memory_offset;
use buddy_slab_allocator::eii::{slab_pool_impl, virt_to_phys_impl};
use buddy_slab_allocator::{GlobalAllocator, PerCpuSlab, SlabPoolTrait, StaticSlabPool};
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageSize, PageTableFlags, Size4KiB};

const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB
const HEAP_VIRT_START: u64 = 0xffff_8800_4000_0000;
const PAGE_SIZE: usize = Size4KiB::SIZE as usize;
const NUM_PAGES: usize = HEAP_SIZE / PAGE_SIZE;

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

fn current_cpu_id() -> usize {
    cpu::per_cpu::get_per_cpu_info().shared.id as usize
}

const SLAB_POOLS: [PerCpuSlab<PAGE_SIZE>; 1] = [PerCpuSlab::new(0)];
static SLAB_POOL: StaticSlabPool<PAGE_SIZE, 1> = StaticSlabPool::new(SLAB_POOLS, current_cpu_id);

#[virt_to_phys_impl]
fn virt_to_phys(vaddr: usize) -> usize {
    let offset = physical_memory_offset() as usize;
    vaddr.checked_sub(offset).expect("virt_to_phys: address below physical offset")
}

#[slab_pool_impl]
fn slab_pool() -> &'static dyn SlabPoolTrait {
    &SLAB_POOL
}

pub(crate) fn init_heap(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> Result<(), &'static str> {
    let mut mapper = active_page_table_mapper();
    let start_virt = VirtAddr::new(HEAP_VIRT_START);

    for i in 0..NUM_PAGES {
        let phys_frame = frame_allocator.allocate_frame().ok_or("Failed to allocate physical frame for heap")?;

        let page = Page::containing_address(start_virt + (i * PAGE_SIZE) as u64);
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL;

        unsafe {
            mapper.map_to(page, phys_frame, flags, frame_allocator).map_err(|_| "Failed to map heap page")?.flush();
        }
    }

    let heap_start = HEAP_VIRT_START as *mut u8;
    let heap_slice = unsafe { core::slice::from_raw_parts_mut(heap_start, HEAP_SIZE) };

    unsafe {
        ALLOCATOR.init(heap_slice).map_err(|_| "Failed to init buddy allocator")?;
    }

    Ok(())
}
