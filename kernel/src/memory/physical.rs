use core::ops::Add;

use bootloader_api::info::MemoryRegions;
use x86_64::{
    PhysAddr, VirtAddr, align_up,
    structures::paging::{FrameAllocator, PageSize, PhysFrame, Size4KiB},
};

use crate::memory::{
    MemoryRegionsExt, frame_allocator::with_frame_allocator, phys_to_virt_mut, physical_memory_offset,
};

/// Reserves a contiguous memory range from the beginning of the first usable
/// memory region that can satisfy the requested size.
///
/// The requested size is rounded up to a 4 KiB page boundary. The selected
/// region's start address is then advanced to exclude the reserved memory,
/// making the reserved range permanently unavailable for future allocations.
///
/// The returned address is the physical start address of the reserved range.
///
/// # Arguments
///
/// * `regions` - Mutable slice of physical memory regions. Usable regions may
///   be modified as memory is reserved.
/// * `size` - Requested reservation size in bytes.
///
/// # Returns
///
/// Returns `Some(physical_address)` containing the start address of the
/// reserved memory on success, or `None` if no usable region is large enough.
///
/// # Note
///
/// This function only reserves memory from the beginning of a usable region.
/// It does not split regions, so memory can only be consumed from region starts.
pub fn reserve_memory(regions: &mut MemoryRegions, size: usize) -> Option<u64> {
    let size = align_up(size as u64, Size4KiB::SIZE);

    for region in regions.usable_regions_mut() {
        let start = align_up(region.start, Size4KiB::SIZE);
        let end = start + size;
        if end > region.end {
            continue;
        }

        region.start = end;
        return Some(start);
    }

    None
}

// pub fn allocate_frame<T>() -> (*mut T, u64) {
//     with_frame_allocator(|alloc| {
//         let frame: PhysFrame<Size4KiB> = alloc.allocate_frame().expect("No frame for xHCI");

//         let phys = frame.start_address().as_u64();
//         let virt = phys_to_virt_mut::<T>(phys);

//         unsafe { virt.write_bytes(0, Size4KiB::SIZE as usize) }

//         (virt, phys)
//     })
// }

#[derive(Debug, Clone, Copy)]
pub struct PhysRegion {
    pub phys: PhysAddr,
    pub virt: VirtAddr,
    pub size: usize,
}
impl PhysRegion {
    pub fn fill_zero(&self) {
        unsafe { self.virt.as_mut_ptr::<u8>().write_bytes(0, self.size) };
    }
    pub fn read_as<T>(&self) -> T {
        unsafe { self.virt.as_mut_ptr::<T>().read_volatile() }
    }
    pub fn write_as<T>(&self, value: T) {
        unsafe { self.virt.as_mut_ptr::<T>().write_volatile(value) }
    }
}

pub fn allocate_frame() -> PhysRegion {
    with_frame_allocator(|alloc| {
        let frame: PhysFrame<Size4KiB> = alloc.allocate_frame().expect("No frame for xHCI");
        const SIZE: usize = Size4KiB::SIZE as usize;
        let phys = frame.start_address();
        let virt = VirtAddr::new(phys.add(physical_memory_offset()).as_u64());

        let phys_region = PhysRegion { phys, virt, size: SIZE };
        phys_region.fill_zero();

        phys_region
    })
}
