use crate::memory::{frame_allocator::with_frame_allocator, phys_to_virt_ref_mut, physical_memory_offset};
use core::{
    ops::Add,
    sync::atomic::{AtomicU64, Ordering},
};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags, PhysFrame, Size4KiB},
};

pub fn active_page_table_mapper() -> OffsetPageTable<'static> {
    let (pml4_frame, _) = Cr3::read();
    let pml4_phys = pml4_frame.start_address().as_u64();
    let pml4 = phys_to_virt_ref_mut::<PageTable>(pml4_phys);
    let offset = VirtAddr::new(physical_memory_offset());
    unsafe { OffsetPageTable::new(pml4, offset) }
}

/// Base address for kernel‑managed virtual mappings (MMIO, etc.)
/// Choose a canonical address in the higher half, e.g., 0xffff_ff80_0000_0000.
const KERNEL_MMIO_BASE: u64 = 0xffff_ff80_0000_0000;

static NEXT_VIRT_ADDR: AtomicU64 = AtomicU64::new(KERNEL_MMIO_BASE);

/// Allocate a virtual address range of `size` bytes.
pub fn alloc_virt_range(size: usize) -> VirtAddr {
    let addr = NEXT_VIRT_ADDR.fetch_add(size as u64, Ordering::Relaxed);
    VirtAddr::new(addr)
}

/// Map a physical memory region to a new virtual address.
fn map_phys_to_virt(phys_base: PhysAddr, size: usize, flags: PageTableFlags) -> VirtAddr {
    let virt_base = alloc_virt_range(size);
    let mut mapper = active_page_table_mapper();

    with_frame_allocator(|alloc| {
        for offset in (0..size as u64).step_by(Size4KiB::SIZE as usize) {
            let phys = phys_base.add(offset);
            let page = Page::<Size4KiB>::containing_address(virt_base + offset as u64);
            let frame = PhysFrame::<Size4KiB>::containing_address(phys);
            unsafe {
                mapper.map_to(page, frame, flags, alloc).expect("Failed to map physical region").flush();
            }
        }
    });
    virt_base
}
pub fn map_mmio(phys_base: PhysAddr, size: usize) -> VirtAddr {
    let flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE
        | PageTableFlags::WRITE_THROUGH
        | PageTableFlags::NO_CACHE;
    map_phys_to_virt(phys_base, size, flags)
}
