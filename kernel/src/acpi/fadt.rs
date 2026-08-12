use crate::{
    acpi::{AcpiGenericAddress, AcpiHeader},
    memory::phys_to_virt_unaligned,
};

#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct FadtTable {
    pub header: AcpiHeader,

    // ACPI 1.0 / 2.0 fields
    pub firmware_ctrl: u32, // Physical address of FACS
    pub dsdt: u32,          // Physical address of DSDT

    pub reserved: u8, // formerly INT_MODEL
    pub preferred_pm_profile: u8,
    pub sci_int: u16,
    pub smi_cmd: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_cnt: u8,
    pub pm1a_evt_blk: u32,
    pub pm1b_evt_blk: u32,
    pub pm1a_cnt_blk: u32,
    pub pm1b_cnt_blk: u32,
    pub pm2_cnt_blk: u32,
    pub pm_tmr_blk: u32,
    pub gpe0_blk: u32,
    pub gpe1_blk: u32,
    pub pm1_evt_len: u8,
    pub pm1_cnt_len: u8,
    pub pm2_cnt_len: u8,
    pub pm_tmr_len: u8,
    pub gpe0_len: u8,
    pub gpe1_len: u8,
    pub gpe1_base: u8,
    pub cst_cnt: u8,
    pub p_lvl2_lat: u16,
    pub p_lvl3_lat: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alrm: u8,
    pub mon_alrm: u8,
    pub century: u8,

    pub iapc_boot_arch: u16,
    pub reserved2: u8,
    pub flags: u32,

    pub reset_reg: AcpiGenericAddress,
    pub reset_value: u8,
    pub reserved3: [u8; 3],

    // 64‑bit extended fields (ACPI 2.0+)
    pub x_firmware_ctrl: u64,
    pub x_dsdt: u64,
    pub x_pm1a_evt_blk: AcpiGenericAddress,
    pub x_pm1b_evt_blk: AcpiGenericAddress,
    pub x_pm1a_cnt_blk: AcpiGenericAddress,
    pub x_pm1b_cnt_blk: AcpiGenericAddress,
    pub x_pm2_cnt_blk: AcpiGenericAddress,
    pub x_pm_tmr_blk: AcpiGenericAddress,
    pub x_gpe0_blk: AcpiGenericAddress,
    pub x_gpe1_blk: AcpiGenericAddress,

    // ACPI 5.0+ sleep registers
    pub sleep_control_reg: AcpiGenericAddress,
    pub sleep_status_reg: AcpiGenericAddress,

    // ACPI 6.0+ hypervisor vendor identity
    pub hypervisor_vendor_identity: u64,
}

#[derive(Debug, Clone)]
pub struct FADT {
    pub table: FadtTable,
}
impl FADT {
    pub fn new(fadt_addr: u64) -> Self {
        let fadt = phys_to_virt_unaligned::<FadtTable>(fadt_addr);
        Self { table: fadt }
    }
}
