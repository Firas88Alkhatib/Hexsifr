use alloc::boxed::Box;
use ap_startup::{Context, platform::Platform, start_all_aps};
use spin::Mutex;
use x86_64::{
    PhysAddr, VirtAddr, align_down, align_up,
    instructions::interrupts,
    structures::paging::{Mapper, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB},
};

use crate::acpi::{acpi_handler::AcpiHandler, get_acpi};
use crate::cpu::per_cpu::{PerCPU, PerCPULocal, get_cpu_info, get_per_cpu_data_by_lapic_id, set_gs_base};
use crate::cpu::{gdt::create_ap_gdt, idt::get_idt, lapic::new_lapic};
use crate::memory::{frame_allocator::with_frame_allocator, mapper::active_page_table_mapper, phys_to_virt_mut};
use crate::scheduler::{self, PerCpuScheduler};
use crate::time::{calibration::calibrate_lapic_timer, sleep_us};
use crate::{task_a, task_b, task_c, task_d};

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
    let current_local_apic = &mut get_cpu_info().local.as_mut().expect("Per CPU local data is not set").lapic;
    let context = Context { acpi_tables: get_acpi(), current_local_apic };
    start_all_aps::<APStartupPlatform, AcpiHandler>(ap_main, context).expect("failed to wake APs");
}

#[unsafe(no_mangle)]
extern "C" fn ap_main() -> ! {
    let mut lapic = new_lapic();
    unsafe {
        lapic.enable();
        lapic.enable_timer();
    }

    let lapic_id = unsafe { lapic.id() };
    let shared = get_per_cpu_data_by_lapic_id(lapic_id).expect("Cannot find PerCPU data for AP");
    // set gs base temporarly for the GDT to be created
    let per_cpu = PerCPU { shared: shared.clone(), local: None, scheduler: Mutex::new(PerCpuScheduler::new()) };

    let raw_ptr = &per_cpu as *const _ as u64;
    set_gs_base(VirtAddr::new(raw_ptr));
    let gdt = create_ap_gdt();
    gdt.load();
    set_gs_base(VirtAddr::new(raw_ptr));

    let local = Some(PerCPULocal { gdt, lapic });
    let per_cpu = PerCPU { local, shared: shared.clone(), scheduler: Mutex::new(PerCpuScheduler::new()) };
    let static_per_cpu = Box::leak(Box::new(per_cpu));
    set_gs_base(VirtAddr::from_ptr(static_per_cpu));
    get_idt().load();

    calibrate_lapic_timer();
    interrupts::enable();

    let id = get_cpu_info().shared.id;
    info!("Initialized AP: {}", id);

    scheduler::init();
    scheduler::spawn(task_a, scheduler::Priority::Normal);
    scheduler::spawn(task_b, scheduler::Priority::High);
    scheduler::spawn(task_c, scheduler::Priority::Low);
    scheduler::spawn(task_d, scheduler::Priority::Normal);
    scheduler::start()
}
