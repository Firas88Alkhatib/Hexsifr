use core::{alloc::Layout, ptr::addr_of};

use alloc::{alloc::alloc_zeroed, boxed::Box};
use spin::Once;
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

pub mod ist {
    pub const NMI: usize = 1;
    pub const DOUBLE_FAULT: usize = 2;
    pub const MACHINE_CHECK: usize = 3;
}

#[derive(Debug)]
pub struct IDTSegmentSelectors {
    pub code_sel: SegmentSelector,
    pub data_sel: SegmentSelector,
    pub user_data_sel: SegmentSelector,
    pub user_code_sel: SegmentSelector,
    pub tss_sel: SegmentSelector,
}

const STACK_SIZE: usize = 4 * Size4KiB::SIZE as usize;

struct InterruptStack(#[allow(dead_code)] [u8; STACK_SIZE]);
static BSP_NMI_STACK: InterruptStack = InterruptStack([0; STACK_SIZE]);
static BSP_DOUBLE_FAULT_STACK: InterruptStack = InterruptStack([0; STACK_SIZE]);
static BSP_MACHINE_CHECK_STACK: InterruptStack = InterruptStack([0; STACK_SIZE]);

static BSP_TSS: Once<TaskStateSegment> = Once::new();
static BSP_SEGS: Once<IDTSegmentSelectors> = Once::new();
static BSP_GDT: Once<GlobalDescriptorTable> = Once::new();

static BSP_PER_CPU_GDT: Once<PerCpuGdt> = Once::new();
#[repr(align(4096))]
#[derive(Debug)]
pub struct PerCpuGdt {
    pub gdt: &'static GlobalDescriptorTable,
    pub tss: &'static TaskStateSegment,
    pub selectors: &'static IDTSegmentSelectors,
}

impl PerCpuGdt {
    /// Loads this GDT and TSS on the current CPU.
    pub fn load(&'static self) {
        self.gdt.load();

        unsafe {
            segmentation::CS::set_reg(self.selectors.code_sel);
            segmentation::SS::set_reg(self.selectors.data_sel);
            segmentation::DS::set_reg(self.selectors.data_sel);
            segmentation::ES::set_reg(self.selectors.data_sel);
            segmentation::FS::set_reg(self.selectors.data_sel);
            segmentation::GS::set_reg(self.selectors.data_sel);
            load_tss(self.selectors.tss_sel);
        }
    }
}

fn get_bsp_tss() -> &'static TaskStateSegment {
    BSP_TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[ist::NMI] = VirtAddr::from_ptr(addr_of!(BSP_NMI_STACK)) + STACK_SIZE as u64;
        tss.interrupt_stack_table[ist::DOUBLE_FAULT] = VirtAddr::from_ptr(addr_of!(BSP_DOUBLE_FAULT_STACK)) + STACK_SIZE as u64;
        tss.interrupt_stack_table[ist::MACHINE_CHECK] = VirtAddr::from_ptr(addr_of!(BSP_MACHINE_CHECK_STACK)) + STACK_SIZE as u64;
        tss
    })
}
fn get_bsp_gdt() -> (&'static GlobalDescriptorTable, &'static IDTSegmentSelectors) {
    let gdt = BSP_GDT.call_once(|| {
        let tss = get_bsp_tss();
        let mut gdt = GlobalDescriptorTable::new();

        let code_sel = gdt.append(Descriptor::kernel_code_segment());
        let data_sel = gdt.append(Descriptor::kernel_data_segment());
        let user_data_sel = gdt.append(Descriptor::user_data_segment());
        let user_code_sel = gdt.append(Descriptor::user_code_segment());
        let tss_sel = gdt.append(Descriptor::tss_segment(tss));

        BSP_SEGS.call_once(|| IDTSegmentSelectors { code_sel, data_sel, user_data_sel, user_code_sel, tss_sel });
        gdt
    });
    (gdt, BSP_SEGS.get().expect("Failed to get GDT Segment Selectors"))
}

pub fn get_bsp_per_cpu_gdt() -> &'static PerCpuGdt {
    BSP_PER_CPU_GDT.call_once(|| {
        let (gdt, selectors) = get_bsp_gdt();
        let tss = get_bsp_tss();
        PerCpuGdt { gdt, selectors, tss }
    })
}

pub fn create_ap_gdt() -> &'static PerCpuGdt {
    let mut tss_box = Box::new(TaskStateSegment::new());

    let nmi_stack = allocate_interrupt_stack();
    let double_fault_stack = allocate_interrupt_stack();
    let machine_check_stack = allocate_interrupt_stack();

    tss_box.interrupt_stack_table[ist::NMI] = VirtAddr::new(nmi_stack.as_ptr() as u64 + STACK_SIZE as u64);
    tss_box.interrupt_stack_table[ist::DOUBLE_FAULT] = VirtAddr::new(double_fault_stack.as_ptr() as u64 + STACK_SIZE as u64);
    tss_box.interrupt_stack_table[ist::MACHINE_CHECK] = VirtAddr::new(machine_check_stack.as_ptr() as u64 + STACK_SIZE as u64);

    let tss = Box::leak(tss_box); // now `'static`

    let mut gdt_box = Box::new(GlobalDescriptorTable::new());
    let code_sel = gdt_box.append(Descriptor::kernel_code_segment());
    let data_sel = gdt_box.append(Descriptor::kernel_data_segment());
    let user_data_sel = gdt_box.append(Descriptor::user_data_segment());
    let user_code_sel = gdt_box.append(Descriptor::user_code_segment());
    let tss_sel = gdt_box.append(Descriptor::tss_segment(tss));

    let gdt = Box::leak(gdt_box);
    // Leak the whole structure to make it static
    let selectors = Box::leak(Box::new(IDTSegmentSelectors { code_sel, data_sel, user_data_sel, user_code_sel, tss_sel }));
    Box::leak(Box::new(PerCpuGdt { gdt, tss, selectors }))
}

fn allocate_interrupt_stack() -> &'static [u8] {
    let layout = Layout::from_size_align(STACK_SIZE, Size4KiB::SIZE as usize).expect("Failed to allocate interrupt stack");
    let ptr = unsafe { alloc_zeroed(layout) };
    let ptr = core::ptr::NonNull::new(ptr).expect("Interrupt stack allocation verification failed");
    unsafe { core::slice::from_raw_parts(ptr.as_ptr(), STACK_SIZE) }
}
