use crate::memory::{phys_to_virt_ref_mut, physical_memory_offset};
use x86_64::{
    VirtAddr,
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
};

pub fn active_page_table_mapper() -> OffsetPageTable<'static> {
    let (pml4_frame, _) = Cr3::read();
    let pml4_phys = pml4_frame.start_address().as_u64();
    let pml4 = phys_to_virt_ref_mut::<PageTable>(pml4_phys);
    let offset = VirtAddr::new(physical_memory_offset());
    unsafe { OffsetPageTable::new(pml4, offset) }
}
