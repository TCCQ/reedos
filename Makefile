# --------------------------------------------------------------------
# When in doubt, run `make all`, and read this file's comments.

#general options
CROSS_COMPILE := riscv64-linux-gnu-
export CROSS_COMPILE
MAKEFLAGS := -j1
export MAKEFLAGS
# ^ causes problems if parallel?

# opensbi options
PLATFORM := generic
export PLATFORM

BOARD := qemu-riscv64_spl


# --------------------------------------------------------------------
# defaults

# This informs uboot that we want a different target than visionfive2
virt-run: fit/virt.itb opensbi/build/platform/generic/firmware/fw_dynamic.bin \
		u-boot/.config u-boot/spl/u-boot-spl.bin u-boot/u-boot.itb
	qemu-system-riscv64 -nographic -machine virt -m 4G -bios u-boot/spl/u-boot-spl.bin \
		-device loader,file=u-boot/u-boot.itb,addr=0x80200000 \
		-device loader,file=fit/virt.itb,addr=0x90000000 \
		-global virtio-mmio.force-legacy=false \
		-drive file=kernel/fs.img,if=none,format=raw,id=x0,read-only=off \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
		${QEMU_FLAGS}

# TODO move fs out of kernel crate

target/riscv64gc-unknown-none-elf/debug/kernel: FORCE
	cargo build

FORCE:

# trailing slash
KERNEL_PATH := target/riscv64gc-unknown-none-elf/debug/

${KERNEL_PATH}kernel.bin: ${KERNEL_PATH}kernel
	${CROSS_COMPILE}objcopy -O binary $< $@

# This is the flat image expected by uboot (the thing you boot), which
# contains a device tree binary that describes the layout of the qemu
# virt machine and the kernel that you are booting. See the .its file
# for the specifics.
fit/virt.itb:  fit/virt.its ${KERNEL_PATH}kernel.bin
	cd fit ; \
	mkimage -f virt.its virt.itb

# This makes and runs a opensbi/uboot boot process on qemu that
# enables you to boot kernel/kernel. The idea is that if the kernel is
# largely hardware independent, then it can be quickly developed with
# this, and shipped with the sd card target. Exit qemu with C-a x.

# --------------------------------------------------------------------
# opensbi stuff

# These are the parts of opensbi that uboot needs. In short this is
# the part of the first stage bootloader that opensbi is responsible
# for. This is a seperate target, as uboot depends on it, but other
# parts of opensbi depend on uboot.
opensbi/build/platform/generic/firmware/fw_dynamic.bin:
	make -C opensbi

# This is the primary uboot binary, wrapped by opensbi to provide the
# expected library features. It is the second stage boot loader, and
# it is what provides the uboot prompt.
opensbi/build/platform/generic/firmware/fw_payload.bin: u-boot/u-boot-dtb.bin
	make -C opensbi FW_PAYLOAD_PATH=../u-boot/u-boot-dtb.bin

# --------------------------------------------------------------------
# u-boot stuff

# What are we targetting? This is the same file location for virt and
# visionfive2, so be sure to run `make clean` when trying to switch
# between targets.
u-boot/.config:
	make -C u-boot ${BOARD}_defconfig

# This is an image for the main uboot binary. It is loaded by qemu to
# provide the second stage bootloader (the prompt). The equivalent
# thing for visionfive2 is u-boot-dtb.bin that gets wrapped by opensbi
# as fw_payload and loaded into memory from a special partition by the
# first stage bootloader.
u-boot/u-boot.itb: opensbi/build/platform/generic/firmware/fw_dynamic.bin u-boot/.config
	cp $< -t u-boot/
	make -C u-boot
#for some reason uboot doesn't have a rule for u-boot.itb, but default makes it anyway?

# This is the primary u-boot image expressed as a binary. It is
# wrapped by opensbi and provides the uboot prompt and acts as the
# second stage bootloader.
u-boot/u-boot-dtb.bin: opensbi/build/platform/generic/firmware/fw_dynamic.bin u-boot/.config
	cp $< -t u-boot/
	make -C u-boot u-boot-dtb.bin
#This target exists for VF2, but not for virt?

# This is the the first stage bootloader expected by uboot. It is a
# wrapper of opensbi and provides other black box stuff for
# uboot. Just think about this as the first stage bootloader, and that
# everything after this is running atop opensbi.
u-boot/spl/u-boot-spl.bin: opensbi/build/platform/generic/firmware/fw_dynamic.bin u-boot/.config
	cp $< u-boot/
	make -C u-boot spl/u-boot-spl.bin

# --------------------------------------------------------------------
# misc

# This just removes all the build artifacts and results. It should be
# run between making different targets, as it removes the u-boot
# config, which is a shared file location between targets.
clean:
	make -C opensbi clean
	make -C u-boot clean
	rm -f fit/virt.itb \
<<<<<<< HEAD
		u-boot/.config \
		${KERNEL_PATH}kernel.bin
=======
		u-boot/.config
>>>>>>> fs-hal-ontop
