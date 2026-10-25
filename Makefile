TARGET  = x86_64-unknown-none
KERNEL  = target/miux.elf
ISO     = MiUX.iso
INITRD  = initrd.tar
CARGO   = cargo +nightly
VGA    ?= std

LLD := $(shell which ld.lld 2>/dev/null || echo $(HOME)/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/ld.lld)

GRUB_MODULES = biosdisk part_msdos part_gpt iso9660 normal \
               multiboot multiboot2 elf configfile \
               all_video vbe vga gfxterm font \
               echo test sleep halt reboot serial

GRUB_X64_EFI  = /usr/lib/grub/x86_64-efi
GRUB_IA32_EFI = /usr/lib/grub/i386-efi
GRUB_EFI_MODULES = part_gpt part_msdos fat iso9660 normal configfile boot \
                   multiboot multiboot2 elf all_video video video_fb efi_gop \
                   gfxterm font echo test sleep halt reboot serial \
                   search search_fs_file search_fs_uuid

.PHONY: all clean run setup help fat32-disk fat32-disk-test run-fat32 run-fat32-debug check-deps

all: check-deps $(ISO)

check-deps:
	@command -v grub-mkrescue >/dev/null 2>&1 || { echo "ERROR: grub-mkrescue not found. Install grub-pc-bin xorriso"; exit 1; }
	@command -v nasm >/dev/null 2>&1 || { echo "ERROR: nasm not found"; exit 1; }
	@command -v xorriso >/dev/null 2>&1 || { echo "ERROR: xorriso not found. Install xorriso"; exit 1; }
	@command -v grub-mkimage >/dev/null 2>&1 || { echo "ERROR: grub-mkimage not found. Install grub-common"; exit 1; }
	@command -v mkfs.vfat >/dev/null 2>&1 || { echo "ERROR: mkfs.vfat not found. Install dosfstools"; exit 1; }
	@command -v mcopy >/dev/null 2>&1 || { echo "ERROR: mcopy/mmd not found. Install mtools"; exit 1; }
	@[ -d $(GRUB_X64_EFI) ] || { echo "ERROR: UEFI GRUB (x86_64-efi) not found. Install grub-efi-amd64-bin"; exit 1; }
	@[ -d $(GRUB_IA32_EFI) ] || echo "WARN: grub-efi-ia32-bin not installed (32-bit UEFI will be skipped)"
	@command -v qemu-system-x86_64 >/dev/null 2>&1 || echo "WARN: qemu not found (run targets will fail)"
	@echo "  Dependencies OK"

$(INITRD):
	@mkdir -p initrd
	@echo "MiUX 5" > initrd/config.txt
	@echo "Welcome!" > initrd/readme.txt
	@echo "mode=normal" > initrd/boot.cfg
	@cd initrd && tar -cf ../$(INITRD) * 2>/dev/null || true
	@echo "  InitRD created"

target/boot.o: src/boot.asm
	@mkdir -p target
	nasm -f elf64 $< -o $@
	@echo "  Bootloader compiled"

CC      ?= gcc
UI_SRC   = $(wildcard src/ui/*.c)
UI_OBJ   = $(patsubst src/ui/%.c,target/ui/%.o,$(UI_SRC))
UI_CFLAGS = -std=gnu11 -O2 -Wall -Wextra -ffreestanding -fno-builtin -nostdlib \
            -fno-stack-protector -fno-pic -fno-pie -mcmodel=kernel -mno-red-zone \
            -mgeneral-regs-only -mno-sse -mno-mmx -mno-80387 \
            -fno-asynchronous-unwind-tables -fno-exceptions -ffunction-sections -fdata-sections

target/ui/%.o: src/ui/%.c src/ui/ui.h
	@mkdir -p target/ui
	$(CC) $(UI_CFLAGS) -c $< -o $@

GRUB_I386 = /usr/lib/grub/i386-pc

target/grub_boot.img: $(GRUB_I386)/boot.img
	@mkdir -p target
	cp $< $@

target/grub_embed.cfg: Makefile
	@mkdir -p target
	@printf '%s\n' \
		'set gfxpayload=keep' \
		'multiboot2 /MIUX/MIUX.ELF' \
		'boot' \
		> $@

target/grub_core.img: target/grub_embed.cfg
	grub-mkimage -O i386-pc -d $(GRUB_I386) -p '(,msdos1)/' -c target/grub_embed.cfg \
		-o $@ biosdisk part_msdos fat multiboot2 boot echo all_video vbe

target/$(TARGET)/release/libmiux.a: FORCE target/grub_boot.img target/grub_core.img
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel -C no-redzone=yes" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@echo "  Rust kernel compiled"

FORCE:

$(KERNEL): target/boot.o $(UI_OBJ) target/$(TARGET)/release/libmiux.a linker.ld
	$(LLD) -T linker.ld -o $@ \
		target/boot.o \
		$(UI_OBJ) \
		target/$(TARGET)/release/libmiux.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  Kernel linked: $(KERNEL)"

$(ISO): $(KERNEL) $(INITRD)
	@rm -rf iso
	@mkdir -p iso/boot/grub/fonts iso/miux
	cp $(KERNEL) iso/boot/miux.elf
	cp $(INITRD) iso/boot/initrd.tar
	@if [ -f /usr/share/grub/unicode.pf2 ]; then \
		cp /usr/share/grub/unicode.pf2 iso/boot/grub/fonts/; \
	fi
	@if [ -f background.png ]; then cp background.png iso/boot/grub/background.png; fi

	@echo "mode=normal" > iso/miux/boot.cfg
	@echo "vesa=true"   >> iso/miux/boot.cfg
	@echo "mode=debug"  > iso/miux/debug.cfg
	@echo "vesa=true"   >> iso/miux/debug.cfg
	@echo "verbose=true">> iso/miux/debug.cfg

	@printf '%s\n' \
		'set timeout=15' \
		'set timeout_style=menu' \
		'set default=0' \
		'' \
		'if loadfont /boot/grub/fonts/unicode.pf2 ; then' \
		'    set gfxmode=1920x1080x24,1920x1080,auto' \
		'    set gfxpayload=keep' \
		'    terminal_output gfxterm' \
		'fi' \
		'' \
		'echo "[BOOT] Welcome to GRUB!"' \
		'echo ""' \
		'' \
		'menuentry "Boot MiUX (LiveCD)" --class miux {' \
		'    echo "[ GRUB ] Load Miux... [ OK ]"' \
		'    multiboot2 /boot/miux.elf' \
		'    initrd /boot/initrd.tar' \
		'    boot' \
		'}' \
		'' \
		'menuentry "Install MiUX (Standart)" --class install {' \
		'    echo "[ GRUB ] Load MiUX Installer... [ OK ]"' \
		'    multiboot2 /boot/miux.elf install' \
		'    module2 /boot/miux.elf miux_payload' \
		'    boot' \
		'}' \
		'' \
		'menuentry "Reboot" --class reboot { reboot }' \
		'menuentry "Shutdown" --class shutdown { halt }' \
		> iso/boot/grub/grub.cfg

	@echo "  Creating hybrid ISO (Legacy BIOS + UEFI)..."
	@set -e; \
	WORK=$$(mktemp -d); trap 'rm -rf "$$WORK"' EXIT; \
	export MTOOLS_SKIP_CHECK=1; \
	mkdir -p iso/boot/grub/i386-pc; \
	cp $(GRUB_I386)/*.mod $(GRUB_I386)/*.lst iso/boot/grub/i386-pc/; \
	grub-mkimage -O i386-pc-eltorito -d $(GRUB_I386) -p /boot/grub \
		-o iso/boot/grub/i386-pc/eltorito.img $(GRUB_MODULES); \
	printf '%s\n' \
		'search --no-floppy --file --set=root /boot/grub/grub.cfg' \
		'set prefix=($$root)/boot/grub' \
		'configfile ($$root)/boot/grub/grub.cfg' \
		> $$WORK/efi_embed.cfg; \
	build_efi() { plat=$$1; dir=$$2; out=$$3; \
		[ -d "$$dir" ] || { echo "  skip $$plat (no $$dir)"; return 1; }; \
		mkdir -p iso/boot/grub/$$plat; \
		cp $$dir/*.mod $$dir/*.lst iso/boot/grub/$$plat/; \
		mods=""; \
		for m in $(GRUB_EFI_MODULES); do \
			if [ -f "$$dir/$$m.mod" ]; then mods="$$mods $$m"; \
			else echo "    (module $$m is missing for $$plat, skipped)"; fi; \
		done; \
		grub-mkimage -O $$plat -d $$dir -p /boot/grub -c $$WORK/efi_embed.cfg \
			-o $$WORK/$$out $$mods; \
	}; \
	build_efi x86_64-efi $(GRUB_X64_EFI) BOOTX64.EFI; \
	build_efi i386-efi $(GRUB_IA32_EFI) BOOTIA32.EFI || true; \
	total=$$(cat $$WORK/*.EFI | wc -c); \
	mb=$$(( total / 1048576 + 4 )); \
	truncate -s $${mb}M $$WORK/efi.img; \
	mkfs.vfat -n MIUX_EFI $$WORK/efi.img >/dev/null; \
	mmd -i $$WORK/efi.img ::/EFI ::/EFI/BOOT; \
	for f in $$WORK/*.EFI; do mcopy -i $$WORK/efi.img $$f ::/EFI/BOOT/$$(basename $$f); done; \
	xorriso -as mkisofs -r -J -V ISOIMAGE \
		--grub2-mbr $(GRUB_I386)/boot_hybrid.img \
		-partition_offset 16 \
		-b boot/grub/i386-pc/eltorito.img -no-emul-boot -boot-load-size 4 \
		-boot-info-table --grub2-boot-info \
		-append_partition 2 C12A7328-F81F-11D2-BA4B-00A0C93EC93B $$WORK/efi.img \
		-appended_part_as_gpt \
		-iso_mbr_part_type EBD0A0A2-B9E5-4433-87C0-68B6B72699C7 \
		-eltorito-alt-boot -e --interval:appended_partition_2:all:: -no-emul-boot \
		-isohybrid-gpt-basdat \
		-o $@ iso
	@echo "  ISO created: $@"

run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -m 1024M -vga $(VGA) -no-reboot

run-fat32: $(ISO)
	qemu-system-x86_64 \
		-drive file=fat32.img,format=raw,if=ide,index=0 \
		-cdrom $(ISO) -boot order=d \
		-m 1024M -vga $(VGA) -no-reboot

run-fat32-debug: $(ISO)
	qemu-system-x86_64 \
		-drive file=fat32.img,format=raw,if=ide,index=0 \
		-cdrom $(ISO) -boot order=d \
		-m 1024M -vga $(VGA) \
		-serial mon:stdio -no-reboot -d int,cpu_reset

run-dual: $(ISO)
	qemu-system-x86_64 \
		-drive file=fat32.img,format=raw,if=ide,index=0 \
		-drive file=$(ISO),format=raw,if=ide,index=1,media=cdrom \
		-boot order=d -m 1024M -vga $(VGA) -no-reboot

run-fat32-gdb: $(ISO)
	qemu-system-x86_64 \
		-drive file=fat32.img,format=raw,if=ide,index=0 \
		-cdrom $(ISO) -boot order=d \
		-m 1024M -vga $(VGA) -s -S -no-reboot

fat32-disk:
	dd if=/dev/zero of=fat32.img bs=1M count=100
	mkfs.vfat -F 32 -n "MIUXFAT" fat32.img
	@echo "  FAT32 disk created: fat32.img"

fat32-disk-test: fat32-disk
	mkdir -p /tmp/fat32mnt
	sudo mount -o loop fat32.img /tmp/fat32mnt
	sudo bash -c 'echo "MiUX Kernel v1.0" > /tmp/fat32mnt/readme.txt'
	sudo bash -c 'echo "Hello from MiUX!" > /tmp/fat32mnt/hello.txt'
	sudo mkdir -p /tmp/fat32mnt/miux /tmp/fat32mnt/boot /tmp/fat32mnt/system /tmp/fat32mnt/usr /tmp/fat32mnt/home
	sudo bash -c 'echo "config" > /tmp/fat32mnt/miux/config.cfg'
	sudo bash -c 'echo "boot"   > /tmp/fat32mnt/boot/boot.cfg'
	sudo bash -c 'echo "system" > /tmp/fat32mnt/system/system.cfg'
	sudo bash -c 'echo "user data" > /tmp/fat32mnt/home/user.txt'
	sudo umount /tmp/fat32mnt

fat32-check:
	mdir -i fat32.img ::/ 2>/dev/null || echo "  (mtools not installed)"

clean:
	cargo clean 2>/dev/null || true
	rm -rf target iso $(ISO) $(INITRD) initrd fat32.img
	@echo "  Cleaned"

setup:
	rustup override set nightly
	rustup component add rust-src
	rustup target add $(TARGET)
	sudo apt-get update
	sudo apt-get install -y grub-pc-bin grub-efi-amd64-bin grub-efi-ia32-bin grub-common xorriso nasm \
		fonts-dejavu fonts-dejavu-extra mtools dosfstools
	@echo "  Setup done"

help:
	@echo "Targets: all run run-fat32 run-fat32-debug run-dual run-fat32-gdb"
	@echo "         fat32-disk fat32-disk-test fat32-check clean setup"
	@echo "Options: VGA=std|vmware|cirrus|qxl (default: std)"