/// This module is conditionally included and contians the HAL backing
/// for the qemu riscv 'virt' machine. It assumes that the kernel is
/// booted in S mode by uboot and is running on top of opensbi.

use super::HAL;

impl HALSerial for HAL {

}
impl HALTimer for HAL {

}
impl HALVM for HAL {

}

impl HALBacking for HAL {

}
