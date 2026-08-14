use ap_startup::{Context, platform::Platform, start_all_aps};
use x86_64::{
    PhysAddr, VirtAddr, align_down, align_up,
    structures::paging::{Mapper, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB},
};

use crate::{
    acpi::{acpi_handler::AcpiHandler, get_acpi},
    cpu::per_cpu::get_per_cpu_info,
    memory::{frame_allocator::with_frame_allocator, mapper::active_page_table_mapper, phys_to_virt_mut},
    time::sleep_us,
};

struct APStartupPlatform;

impl Platform for APStartupPlatform {
    const STACK_SIZE: usize = (32 * Size4KiB::SIZE) as usize;

    fn sleep_us(microseconds: u64) {
        sleep_us(microseconds);
    }

    fn phys_to_ptr<T>(phys_addr: u64) -> *mut T {
        phys_to_virt_mut(phys_addr)
    }

    fn map_memory(virt_addr: u64, phys_addr: u64, size: u64) {
        let start_virt = VirtAddr::new(align_down(virt_addr, Size4KiB::SIZE));
        let end_virt = VirtAddr::new(align_up(virt_addr + size, Size4KiB::SIZE));
        let start_phys = PhysAddr::new(align_down(phys_addr, Size4KiB::SIZE));

        let mut mapper = active_page_table_mapper();

        let start_page = Page::<Size4KiB>::containing_address(start_virt);
        let end_page = Page::<Size4KiB>::containing_address(end_virt); // exclusive

        with_frame_allocator(|alloc| {
            for (i, page) in Page::<Size4KiB>::range(start_page, end_page).enumerate() {
                let phys = start_phys + (i as u64) * Size4KiB::SIZE;
                let frame = PhysFrame::<Size4KiB>::containing_address(phys);
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL;
                unsafe {
                    mapper.map_to(page, frame, flags, alloc).expect("Mapping AP stack failed").flush();
                }
            }
        });
    }
}

pub fn start_ap_processors() {
    info!("Starting AP processors");
    let context = Context { acpi_tables: get_acpi(), current_local_apic: &mut get_per_cpu_info().local.lapic };
    start_all_aps::<APStartupPlatform, AcpiHandler>(ap_main, context).expect("failed to wake APs");
}

#[unsafe(no_mangle)]
extern "C" fn ap_main() -> ! {
    // AP entry point.
    // This function runs on each AP after it starts.
    // You need to set up per‑CPU data, load GDT, enable interrupts, etc.
    // info!("AP processor started up");
    info!("Hello from AP");

    loop {
        x86_64::instructions::hlt();
    }
}
