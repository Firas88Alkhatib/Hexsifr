use alloc::{
    collections::{BTreeMap, btree_map::Entry},
    vec::Vec,
};

//
// HID item parsing
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidItemType {
    Main,
    Global,
    Local,
    Reserved,
}

#[derive(Debug, Clone, Copy)]
pub struct HidItem {
    pub item_type: HidItemType,
    pub tag: u8,
    pub value: u32,
    pub size: usize,
}

#[derive(Debug)]
pub enum HidParseError {
    UnexpectedEnd,
    InvalidLongItem,
    GlobalStackUnderflow,
    CollectionUnderflow,
    InvalidReportId,
    InvalidReportField,
}

pub struct HidItemParser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> HidItemParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn next(&mut self) -> Result<Option<HidItem>, HidParseError> {
        if self.offset >= self.data.len() {
            return Ok(None);
        }

        let prefix = self.data[self.offset];
        self.offset += 1;

        //
        // Long item:
        // 0xFE, size, tag, data...
        //
        if prefix == 0xFE {
            let size = *self.data.get(self.offset).ok_or(HidParseError::UnexpectedEnd)? as usize;

            self.offset += 1;

            // Long-item tag.
            let _tag = *self.data.get(self.offset).ok_or(HidParseError::UnexpectedEnd)?;

            self.offset += 1;

            if self.offset + size > self.data.len() {
                return Err(HidParseError::UnexpectedEnd);
            }

            self.offset += size;

            return Err(HidParseError::InvalidLongItem);
        }

        //
        // Short item size:
        //
        // 00 = 0 bytes
        // 01 = 1 byte
        // 10 = 2 bytes
        // 11 = 4 bytes
        //
        let size = match prefix & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 4,
            _ => unreachable!(),
        };

        let item_type = match (prefix >> 2) & 0x03 {
            0 => HidItemType::Main,
            1 => HidItemType::Global,
            2 => HidItemType::Local,
            _ => HidItemType::Reserved,
        };

        let tag = prefix >> 4;

        if self.offset + size > self.data.len() {
            return Err(HidParseError::UnexpectedEnd);
        }

        let bytes = &self.data[self.offset..self.offset + size];
        self.offset += size;

        let value = match size {
            0 => 0,
            1 => bytes[0] as u32,
            2 => u16::from_le_bytes([bytes[0], bytes[1]]) as u32,
            4 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            _ => unreachable!(),
        };

        Ok(Some(HidItem { item_type, tag, value, size }))
    }
}

//
// Final parsed representation
//

#[derive(Debug, Default)]
pub struct HidReportDescriptor {
    pub reports: BTreeMap<u8, HidReport>,
}

#[derive(Debug, Default)]
pub struct HidReport {
    pub id: u8,

    pub inputs: Vec<HidField>,
    pub outputs: Vec<HidField>,
    pub features: Vec<HidField>,

    // Total size of each report type in bits.
    pub input_bits: usize,
    pub output_bits: usize,
    pub feature_bits: usize,
}

impl HidReport {
    pub fn input_report_length(&self) -> usize {
        self.input_bits.div_ceil(8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidFieldKind {
    Input,
    Output,
    Feature,
}

#[derive(Debug, Clone, Copy)]
pub struct HidUsage {
    pub page: u16,
    pub id: u32,
}

#[derive(Debug, Clone)]
pub enum HidUsages {
    List(Vec<HidUsage>),

    Range { page: u16, min: u32, max: u32 },
}

#[derive(Debug, Clone)]
pub struct HidField {
    pub kind: HidFieldKind,

    pub usages: HidUsages,

    // Position inside the report.
    pub bit_offset: usize,
    pub bit_size: usize,
    pub count: usize,

    // Global attributes.
    pub logical_min: i32,
    pub logical_max: i32,
    pub physical_min: i32,
    pub physical_max: i32,

    pub unit_exponent: i8,
    pub unit: u32,

    pub flags: HidFieldFlags,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HidFieldFlags {
    pub constant: bool,
    pub variable: bool,
    pub relative: bool,
    pub wrap: bool,
    pub non_linear: bool,
    pub no_preferred: bool,
    pub null_state: bool,
    pub volatile: bool,
    pub buffered_bytes: bool,
}

impl HidFieldFlags {
    pub fn from_main_item(value: u32) -> Self {
        Self {
            constant: value & (1 << 0) != 0,
            variable: value & (1 << 1) != 0,
            relative: value & (1 << 2) != 0,
            wrap: value & (1 << 3) != 0,
            non_linear: value & (1 << 4) != 0,
            no_preferred: value & (1 << 5) != 0,
            null_state: value & (1 << 6) != 0,
            volatile: value & (1 << 7) != 0,
            buffered_bytes: value & (1 << 8) != 0,
        }
    }
}

//
// Parser state
//

#[derive(Debug, Clone)]
struct GlobalState {
    usage_page: u16,

    logical_min: i32,
    logical_max: i32,

    physical_min: i32,
    physical_max: i32,

    unit_exponent: i8,
    unit: u32,

    report_size: usize,
    report_count: usize,

    report_id: u8,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            usage_page: 0,

            logical_min: 0,
            logical_max: 0,

            physical_min: 0,
            physical_max: 0,

            unit_exponent: 0,
            unit: 0,

            report_size: 0,
            report_count: 0,

            report_id: 0,
        }
    }
}

#[derive(Debug, Default)]
struct LocalState {
    usages: Vec<HidUsage>,

    usage_min: Option<HidUsage>,
    usage_max: Option<HidUsage>,
}

impl LocalState {
    fn clear(&mut self) {
        self.usages.clear();
        self.usage_min = None;
        self.usage_max = None;
    }
}

//
// Main parser
//

pub struct HidParser {
    global: GlobalState,
    global_stack: Vec<GlobalState>,
    local: LocalState,

    collection_depth: usize,

    pub descriptor: HidReportDescriptor,
}

impl HidParser {
    pub fn new() -> Self {
        Self {
            global: GlobalState::default(),
            global_stack: Vec::new(),
            local: LocalState::default(),
            collection_depth: 0,
            descriptor: HidReportDescriptor::default(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<HidReportDescriptor, HidParseError> {
        let mut parser = Self::new();
        let mut items = HidItemParser::new(data);

        while let Some(item) = items.next()? {
            parser.process(item)?;
        }

        if parser.collection_depth != 0 {
            return Err(HidParseError::CollectionUnderflow);
        }

        Ok(parser.descriptor)
    }

    fn process(&mut self, item: HidItem) -> Result<(), HidParseError> {
        match item.item_type {
            HidItemType::Global => self.process_global(item)?,
            HidItemType::Local => self.process_local(item)?,
            HidItemType::Main => self.process_main(item)?,
            HidItemType::Reserved => {}
        }

        Ok(())
    }

    //
    // Global items
    //
    fn process_global(&mut self, item: HidItem) -> Result<(), HidParseError> {
        match item.tag {
            0x00 => {
                // Usage Page
                self.global.usage_page = item.value as u16;
            }

            0x01 => {
                // Logical Minimum
                self.global.logical_min = signed_value(item.value, item.size);
            }

            0x02 => {
                // Logical Maximum
                self.global.logical_max = signed_value(item.value, item.size);
            }

            0x03 => {
                // Physical Minimum
                self.global.physical_min = signed_value(item.value, item.size);
            }

            0x04 => {
                // Physical Maximum
                self.global.physical_max = signed_value(item.value, item.size);
            }

            0x05 => {
                // Unit Exponent
                self.global.unit_exponent = signed_value(item.value, item.size) as i8;
            }

            0x06 => {
                // Unit
                self.global.unit = item.value;
            }

            0x07 => {
                // Report Size
                self.global.report_size = item.value as usize;
            }

            0x08 => {
                // Report ID
                let id = item.value as u8;

                if id == 0 {
                    return Err(HidParseError::InvalidReportId);
                }

                self.global.report_id = id;
                self.ensure_report(id);
            }

            0x09 => {
                // Report Count
                self.global.report_count = item.value as usize;
            }

            0x0A => {
                // Push
                self.global_stack.push(self.global.clone());
            }

            0x0B => {
                // Pop
                self.global = self.global_stack.pop().ok_or(HidParseError::GlobalStackUnderflow)?;
            }

            _ => {}
        }

        Ok(())
    }

    //
    // Local items
    //
    fn process_local(&mut self, item: HidItem) -> Result<(), HidParseError> {
        match item.tag {
            0x00 => {
                // Usage
                self.local.usages.push(self.parse_usage(item));
            }

            0x01 => {
                // Usage Minimum
                self.local.usage_min = Some(self.parse_usage(item));
            }

            0x02 => {
                // Usage Maximum
                self.local.usage_max = Some(self.parse_usage(item));
            }

            _ => {}
        }

        Ok(())
    }

    //
    // Main items
    //
    fn process_main(&mut self, item: HidItem) -> Result<(), HidParseError> {
        match item.tag {
            0x08 => {
                // Input
                self.add_field(HidFieldKind::Input, item.value)?;
            }

            0x09 => {
                // Output
                self.add_field(HidFieldKind::Output, item.value)?;
            }

            0x0A => {
                // Collection
                self.collection_depth += 1;
            }

            0x0B => {
                // Feature
                self.add_field(HidFieldKind::Feature, item.value)?;
            }

            0x0C => {
                // End Collection
                if self.collection_depth == 0 {
                    return Err(HidParseError::CollectionUnderflow);
                }

                self.collection_depth -= 1;
            }

            _ => {}
        }

        //
        // Local items apply only to the next Main item.
        //
        self.local.clear();

        Ok(())
    }

    fn add_field(&mut self, kind: HidFieldKind, raw_flags: u32) -> Result<(), HidParseError> {
        if self.global.report_size == 0 || self.global.report_count == 0 {
            return Err(HidParseError::InvalidReportField);
        }

        let report_id = self.global.report_id;

        self.ensure_report(report_id);

        let bit_offset = self.current_bit_offset(report_id, kind);

        let field = HidField {
            kind,

            usages: self.current_usages(),

            bit_offset,
            bit_size: self.global.report_size,
            count: self.global.report_count,

            logical_min: self.global.logical_min,
            logical_max: self.global.logical_max,

            physical_min: self.global.physical_min,
            physical_max: self.global.physical_max,

            unit_exponent: self.global.unit_exponent,
            unit: self.global.unit,

            flags: HidFieldFlags::from_main_item(raw_flags),
        };

        let field_bits = self.global.report_size * self.global.report_count;

        let report = self.descriptor.reports.get_mut(&report_id).ok_or(HidParseError::InvalidReportId)?;

        match kind {
            HidFieldKind::Input => {
                report.inputs.push(field);
                report.input_bits = bit_offset + field_bits;
            }

            HidFieldKind::Output => {
                report.outputs.push(field);
                report.output_bits = bit_offset + field_bits;
            }

            HidFieldKind::Feature => {
                report.features.push(field);
                report.feature_bits = bit_offset + field_bits;
            }
        }

        Ok(())
    }

    fn current_bit_offset(&self, report_id: u8, kind: HidFieldKind) -> usize {
        let report = self.descriptor.reports.get(&report_id).expect("report must exist");

        let (has_fields, bits) = match kind {
            HidFieldKind::Input => (!report.inputs.is_empty(), report.input_bits),

            HidFieldKind::Output => (!report.outputs.is_empty(), report.output_bits),

            HidFieldKind::Feature => (!report.features.is_empty(), report.feature_bits),
        };

        //
        // If this report has a Report ID, the first byte is the ID.
        //
        if !has_fields && report_id != 0 { 8 } else { bits }
    }

    fn current_usages(&self) -> HidUsages {
        if !self.local.usages.is_empty() {
            return HidUsages::List(self.local.usages.clone());
        }

        if let (Some(min), Some(max)) = (self.local.usage_min, self.local.usage_max) {
            //
            // Usage ranges must belong to the same page.
            // The common HID descriptors, including keyboards,
            // satisfy this.
            //
            return HidUsages::Range { page: min.page, min: min.id, max: max.id };
        }

        HidUsages::List(Vec::new())
    }

    fn parse_usage(&self, item: HidItem) -> HidUsage {
        //
        // For a 4-byte Usage:
        //
        // upper 16 bits = usage page
        // lower 16 bits = usage ID
        //
        // For 1/2-byte Usage:
        //
        // current Usage Page supplies the page.
        //
        if item.size == 4 {
            HidUsage { page: (item.value >> 16) as u16, id: item.value & 0xFFFF }
        } else {
            HidUsage { page: self.global.usage_page, id: item.value }
        }
    }

    fn ensure_report(&mut self, id: u8) {
        match self.descriptor.reports.entry(id) {
            Entry::Occupied(_) => {}

            Entry::Vacant(entry) => {
                entry.insert(HidReport {
                    id,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    features: Vec::new(),
                    input_bits: 0,
                    output_bits: 0,
                    feature_bits: 0,
                });
            }
        }
    }
}

//
// Signed HID value helper
//

fn signed_value(value: u32, size: usize) -> i32 {
    match size {
        0 => 0,
        1 => (value as u8 as i8) as i32,
        2 => (value as u16 as i16) as i32,
        4 => value as i32,
        _ => value as i32,
    }
}
