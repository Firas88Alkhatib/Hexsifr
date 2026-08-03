use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    align_up,
    structures::paging::{PageSize, Size4KiB},
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

    for region in regions.iter_mut() {
        if region.kind != MemoryRegionKind::Usable {
            continue;
        }

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
