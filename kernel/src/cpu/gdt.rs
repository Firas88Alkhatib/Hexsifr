use core::alloc::Layout;

use alloc::{alloc::alloc_zeroed, boxed::Box};
use x86_64::{
    VirtAddr,
    instructions::tables::load_tss,
    registers::segmentation::{self, Segment},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        paging::{PageSize, Size4KiB},
        tss::TaskStateSegment,
    },
};

const STACK_SIZE: usize = 4 * Size4KiB::SIZE as usize;

pub mod ist {
    pub const NMI: usize = 1;
    pub const DOUBLE_FAULT: usize = 2;
    pub const MACHINE_CHECK: usize = 3;
}

pub struct PerCpuGdt {
    pub gdt: GlobalDescriptorTable,
    pub tss: &'static TaskStateSegment,
    pub code_sel: SegmentSelector,
    pub data_sel: SegmentSelector,
    pub user_data_sel: SegmentSelector,
    pub user_code_sel: SegmentSelector,
    pub tss_sel: SegmentSelector,
}

impl PerCpuGdt {
    pub fn new() -> &'static mut Self {
        let mut tss_box = Box::new(TaskStateSegment::new());

        let nmi_stack = allocate_interrupt_stack();
        let double_fault_stack = allocate_interrupt_stack();
        let machine_check_stack = allocate_interrupt_stack();

        tss_box.interrupt_stack_table[ist::NMI] = VirtAddr::new(nmi_stack.as_ptr() as u64 + STACK_SIZE as u64);
        tss_box.interrupt_stack_table[ist::DOUBLE_FAULT] = VirtAddr::new(double_fault_stack.as_ptr() as u64 + STACK_SIZE as u64);
        tss_box.interrupt_stack_table[ist::MACHINE_CHECK] = VirtAddr::new(machine_check_stack.as_ptr() as u64 + STACK_SIZE as u64);

        let tss = Box::leak(tss_box); // now `'static`

        let mut gdt = GlobalDescriptorTable::new();
        let code_sel = gdt.append(Descriptor::kernel_code_segment());
        let data_sel = gdt.append(Descriptor::kernel_data_segment());
        let user_data_sel = gdt.append(Descriptor::user_data_segment());
        let user_code_sel = gdt.append(Descriptor::user_code_segment());
        let tss_sel = gdt.append(Descriptor::tss_segment(tss));

        // Leak the whole structure to make it static
        Box::leak(Box::new(Self { gdt, tss, code_sel, data_sel, user_data_sel, user_code_sel, tss_sel }))
    }

    /// Loads this GDT and TSS on the current CPU.
    pub fn load(&'static self) {
        self.gdt.load();

        unsafe {
            segmentation::CS::set_reg(self.code_sel);
            segmentation::SS::set_reg(self.data_sel);
            segmentation::DS::set_reg(self.data_sel);
            segmentation::ES::set_reg(self.data_sel);
            segmentation::FS::set_reg(self.data_sel);
            segmentation::GS::set_reg(self.data_sel);
            load_tss(self.tss_sel);
        }
    }
}

fn allocate_interrupt_stack() -> &'static mut [u8] {
    let layout = Layout::from_size_align(STACK_SIZE, Size4KiB::SIZE as usize).expect("Failed to allocate interrupt stack");
    let ptr = unsafe { alloc_zeroed(layout) };
    let ptr = core::ptr::NonNull::new(ptr).expect("Interrupt stack allocation verification failed");
    unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr(), STACK_SIZE) }
}
