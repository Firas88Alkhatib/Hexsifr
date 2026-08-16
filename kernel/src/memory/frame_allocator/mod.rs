use bootloader_api::info::MemoryRegions;
use core::slice::from_raw_parts_mut;
use llfree::{Alloc, Class, Classing, FrameId, Init, LLFree, MetaData, MetaSize, Request};
use spin::Mutex;
use x86_64::{
    PhysAddr, align_down, align_up,
    structures::paging::{FrameAllocator, FrameDeallocator, PageSize, PhysFrame, Size2MiB, Size4KiB},
};

use crate::{
    acpi::get_cpu_count,
    cpu,
    memory::{MemoryRegionsExt, phys_to_virt, physical::reserve_memory},
};

static FRAME_ALLOCATOR: Mutex<Option<LLFreeFrameAllocator>> = Mutex::new(None);

pub trait PageSizeOrder: PageSize {
    const ORDER: usize;
    const CLASS: Class;
}

impl PageSizeOrder for Size4KiB {
    const ORDER: usize = 0;
    const CLASS: Class = Class(0);
}

impl PageSizeOrder for Size2MiB {
    const ORDER: usize = 9;
    const CLASS: Class = Class(1);
}

pub struct LLFreeFrameAllocator<'a> {
    inner: LLFree<'a>,
    base_addr: u64,
}

unsafe impl<'a> FrameAllocator<Size4KiB> for LLFreeFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate::<Size4KiB>()
    }
}

impl<'a> FrameDeallocator<Size4KiB> for LLFreeFrameAllocator<'a> {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.deallocate::<Size4KiB>(frame);
    }
}

unsafe impl<'a> FrameAllocator<Size2MiB> for LLFreeFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size2MiB>> {
        self.allocate::<Size2MiB>()
    }
}

impl<'a> FrameDeallocator<Size2MiB> for LLFreeFrameAllocator<'a> {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size2MiB>) {
        self.deallocate::<Size2MiB>(frame);
    }
}

impl LLFreeFrameAllocator<'_> {
    pub fn new(memory_regions: &mut MemoryRegions, num_cpus: usize) -> Result<Self, &'static str> {
        let (base_addr, end_addr) = memory_regions.usable_regions_boudry();

        let total_frames = frame_num::<Size4KiB>(end_addr, base_addr);

        let (classing, _request) = Classing::simple(num_cpus);
        let meta_sizes = LLFree::metadata_size(&classing, total_frames);

        let meta_data = alloc_meta_data(memory_regions, meta_sizes);

        let alloc = LLFree::new(total_frames, Init::AllocAll, &classing, meta_data).expect("Failed to create new LLFree");

        for region in memory_regions.usable_regions() {
            let start_aligned = align_up(region.start, Size4KiB::SIZE);
            let end_aligned = align_down(region.end, Size4KiB::SIZE);
            let start_frame = frame_num::<Size4KiB>(start_aligned, base_addr);
            let end_frame = frame_num::<Size4KiB>(end_aligned, base_addr);
            for frame in start_frame..end_frame {
                // Request doesn't matter during init – use order=0, class=0, local=None
                let req = Request { order: 0, class: Class(0), local: None };

                alloc.put(FrameId(frame), req).map_err(|_| "Failed to free frame")?;
            }
        }

        Ok(Self { inner: alloc, base_addr })
    }

    fn allocate<PS: PageSizeOrder>(&self) -> Option<PhysFrame<PS>> {
        let request = Request { order: PS::ORDER, class: PS::CLASS, local: Some(current_cpu_id()) };

        let (frame, _) = self.inner.get(None, request).ok()?;
        let phys = self.base_addr + frame.0 as u64 * PS::SIZE;
        Some(PhysFrame::containing_address(PhysAddr::new(phys)))
    }

    fn deallocate<PS: PageSizeOrder>(&self, phys: PhysFrame<PS>) {
        let frame_idx = frame_num::<PS>(phys.start_address().as_u64(), self.base_addr);
        let request = Request { order: PS::ORDER, class: PS::CLASS, local: Some(current_cpu_id()) };
        self.inner.put(FrameId(frame_idx), request).unwrap();
    }
}

fn alloc_meta_data(memory_regions: &mut MemoryRegions, meta_sizes: MetaSize) -> MetaData<'static> {
    const ALIGN: usize = 64;

    let trees_offset = llfree::util::align_up(meta_sizes.local, ALIGN);
    let lower_offset = llfree::util::align_up(trees_offset + meta_sizes.trees, ALIGN);
    let total_meta_bytes = lower_offset + meta_sizes.lower;

    let meta_phys = reserve_memory(memory_regions, total_meta_bytes).expect("Metadata reservation failed");

    let base_virt = phys_to_virt::<u8>(meta_phys) as *mut u8;

    let local_slice = unsafe { from_raw_parts_mut(base_virt.add(0), meta_sizes.local) };
    let trees_slice = unsafe { from_raw_parts_mut(base_virt.add(trees_offset), meta_sizes.trees) };
    let lower_slice = unsafe { from_raw_parts_mut(base_virt.add(lower_offset), meta_sizes.lower) };
    MetaData { local: local_slice, trees: trees_slice, lower: lower_slice }
}

#[inline]
fn frame_num<PS: PageSize>(addr: u64, base_addr: u64) -> usize {
    ((addr - base_addr) / PS::SIZE) as usize
}

fn current_cpu_id() -> usize {
    cpu::per_cpu::get_cpu_info().shared.id as usize
}

pub fn init_frame_allocator(memory_regions: &mut MemoryRegions) {
    let num_cpus = get_cpu_count();
    let frame_allocator = LLFreeFrameAllocator::new(memory_regions, num_cpus).expect("Cannot init frame allocator LLFree");
    *FRAME_ALLOCATOR.lock() = Some(frame_allocator);
}
pub fn with_frame_allocator<F, R>(f: F) -> R
where
    F: FnOnce(&mut LLFreeFrameAllocator) -> R,
{
    let mut guard = FRAME_ALLOCATOR.lock();
    let alloc = guard.as_mut().expect("Frame allocator not initialised");
    f(alloc)
}
