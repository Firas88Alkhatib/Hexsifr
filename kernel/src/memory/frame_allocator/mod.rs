use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use core::slice::from_raw_parts_mut;
use llfree::{Alloc, Class, Classing, FrameId, Init, LLFree, MetaData, MetaSize, Request, util::align_up};
use x86_64::{
    PhysAddr,
    structures::paging::{FrameAllocator, FrameDeallocator, PageSize, PhysFrame, Size2MiB, Size4KiB},
};

use crate::memory::{phys_to_virt, physical::reserve_memory};

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
        let (base_addr, end_addr) = get_regions_boudary(memory_regions);

        let total_frames = frame_num::<Size4KiB>(end_addr, base_addr);

        let (classing, _request) = Classing::simple(num_cpus);
        let meta_sizes = LLFree::metadata_size(&classing, total_frames);

        let meta_data = alloc_meta_data(memory_regions, meta_sizes);

        let alloc = LLFree::new(total_frames, Init::AllocAll, &classing, meta_data).expect("Failed to create new LLFree");

        for region in memory_regions.iter() {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }
            let start_frame = frame_num::<Size4KiB>(region.start, base_addr);
            let end_frame = frame_num::<Size4KiB>(region.end, base_addr);
            for frame in start_frame..end_frame {
                // Request doesn't matter during init – use order=0, class=0, local=None
                let req = Request { order: 0, class: Class(0), local: None };

                alloc.put(FrameId(frame), req).map_err(|_| "Failed to free frame")?;
            }
        }

        Ok(Self { inner: alloc, base_addr })
    }

    pub fn allocate<PS: PageSizeOrder>(&self) -> Option<PhysFrame<PS>> {
        let cpu = current_cpu_id();
        let request = Request { order: PS::ORDER, class: PS::CLASS, local: Some(cpu) };

        let (frame, _) = self.inner.get(None, request).ok()?;
        let phys = self.base_addr + frame.0 as u64 * PS::SIZE;
        Some(PhysFrame::containing_address(PhysAddr::new(phys)))
    }

    pub fn deallocate<PS: PageSizeOrder>(&self, phys: PhysFrame<PS>) {
        let frame_idx = frame_num::<PS>(phys.start_address().as_u64(), self.base_addr);
        let request = Request { order: PS::ORDER, class: PS::CLASS, local: Some(current_cpu_id()) };
        self.inner.put(FrameId(frame_idx), request).unwrap();
    }
}

fn alloc_meta_data(memory_regions: &mut MemoryRegions, meta_sizes: MetaSize) -> MetaData<'static> {
    const ALIGN: usize = 64;

    let trees_offset = align_up(meta_sizes.local, ALIGN);
    let lower_offset = align_up(trees_offset + meta_sizes.trees, ALIGN);
    let total_meta_bytes = lower_offset + meta_sizes.lower;

    let meta_phys = reserve_memory(memory_regions, total_meta_bytes).expect("Metadata reservation failed");

    let base_virt = phys_to_virt::<u8>(meta_phys) as *mut u8;

    let local_slice = unsafe { from_raw_parts_mut(base_virt.add(0), meta_sizes.local) };
    let trees_slice = unsafe { from_raw_parts_mut(base_virt.add(trees_offset), meta_sizes.trees) };
    let lower_slice = unsafe { from_raw_parts_mut(base_virt.add(lower_offset), meta_sizes.lower) };
    MetaData { local: local_slice, trees: trees_slice, lower: lower_slice }
}
fn get_regions_boudary(memory_regions: &MemoryRegions) -> (u64, u64) {
    let mut min_start = u64::MAX;
    let mut max_end = 0u64;

    for region in memory_regions.iter() {
        if region.kind == MemoryRegionKind::Usable {
            if region.start < min_start {
                min_start = region.start;
            }
            if region.end > max_end {
                max_end = region.end;
            }
        }
    }
    (min_start, max_end)
}

#[inline]
fn frame_num<PS: PageSize>(addr: u64, base_addr: u64) -> usize {
    ((addr - base_addr) / PS::SIZE) as usize
}

fn current_cpu_id() -> usize {
    // TODO: implement per‑CPU ID retrieval
    0
}
