use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use core::slice::from_raw_parts_mut;
use llfree::{Alloc, Class, Classing, FrameId, Init, LLFree, MetaData, MetaSize, Request, util::align_up};
use x86_64::structures::paging::{PageSize, Size4KiB};

use crate::memory::{phys_to_virt, physical::reserve_memory};

pub struct FrameAllocator<'a> {
    inner: LLFree<'a>,
    base_addr: u64,
}

impl FrameAllocator<'_> {
    pub fn new(memory_regions: &mut MemoryRegions, num_cpus: usize) -> Result<Self, &'static str> {
        let (base_addr, end_addr) = get_regions_boudary(memory_regions);

        let total_frames = frame_num(end_addr, base_addr);

        let (classing, _request) = Classing::simple(num_cpus);
        let meta_sizes = LLFree::metadata_size(&classing, total_frames);

        let meta_data = alloc_meta_data(memory_regions, meta_sizes);

        let alloc = LLFree::new(total_frames, Init::AllocAll, &classing, meta_data).expect("Failed to create new LLFree");

        for region in memory_regions.iter() {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }
            let start_frame = frame_num(region.start, base_addr);
            let end_frame = frame_num(region.end, base_addr);
            for frame in start_frame..end_frame {
                // Request doesn't matter during init – use order=0, class=0, local=None
                let req = Request {
                    order: 0,
                    class: Class(0),
                    local: None,
                };

                alloc.put(FrameId(frame), req).map_err(|_| "Failed to free frame")?;
            }
        }

        Ok(Self { inner: alloc, base_addr })
    }
    pub fn allocate(&self, order: usize) -> Option<u64> {
        let (frame, _) = self.inner.get(None, make_request(order)).ok()?;
        // frame is FrameId, retrieve inner usize
        let frame_idx = frame.0;
        Some(self.base_addr + frame_idx as u64 * Size4KiB::SIZE)
    }

    pub fn deallocate(&self, phys: u64, order: usize) {
        let frame_idx = ((phys - self.base_addr) / Size4KiB::SIZE) as usize;
        let frame_id = FrameId(frame_idx);

        self.inner.put(frame_id, make_request(order)).unwrap();
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
    MetaData {
        local: local_slice,
        trees: trees_slice,
        lower: lower_slice,
    }
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
fn frame_num(addr: u64, base_addr: u64) -> usize {
    ((addr - base_addr) / Size4KiB::SIZE) as usize
}

fn make_request(order: usize) -> Request {
    let cpu = current_cpu_id();
    let class = if order < 9 { Class(0) } else { Class(1) };
    Request {
        order,
        class,
        local: Some(cpu),
    }
}

fn current_cpu_id() -> usize {
    // TODO: implement per‑CPU ID retrieval
    0
}
