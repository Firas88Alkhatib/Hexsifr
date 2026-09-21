#[macro_use]
pub mod serial;
pub mod pci;
pub mod usb;

pub fn pci_init() {
    let pci_devices = pci::get_pci_devices();
    let xhci_device = pci_devices.serial_bus.usb.xhci.first().unwrap();
    usb::xhci::xhci_init(xhci_device.clone());
}
