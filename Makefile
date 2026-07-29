TARGET = x86_64-unknown-none
KERNEL = target/kernel.bin
ISO = MiOS.iso
INITRD = initrd.tar
CARGO = cargo +nightly
VGA ?= vmware

LLD := $(shell which ld.lld 2>/dev/null || echo $(HOME)/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/ld.lld)

.PHONY: all clean run setup

all: $(ISO)

# Создание initrd
initrd:
	@mkdir -p initrd
	@echo "MiOS 5" > initrd/config.txt
	@echo "Welcome!" > initrd/readme.txt
	@cd initrd && tar -cf ../$(INITRD) * 2>/dev/null || true
	@echo "  InitRD created"

# Компиляция загрузчика
target/boot.o: src/boot.asm
	@mkdir -p target
	nasm -f elf64 $< -o $@
	@echo "  Bootloader compiled"

# Компиляция ядра
target/$(TARGET)/release/libmios.a: FORCE
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@echo "  Rust kernel compiled"

FORCE:

# Линковка ядра
$(KERNEL): target/boot.o target/$(TARGET)/release/libmios.a linker.ld
	$(LLD) -T linker.ld -o $@ \
		target/boot.o \
		target/$(TARGET)/release/libmios.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  Kernel linked"

# BIOS ISO
$(ISO): $(KERNEL) initrd
	@mkdir -p iso/boot/grub
	@mkdir -p iso/mios
	cp $(KERNEL) iso/boot/
	cp $(INITRD) iso/boot/
	@if [ -f background.png ]; then cp background.png iso/boot/grub/background.png; fi
	@echo "mode=normal" > iso/mios/boot.cfg
	@echo "vesa=true" >> iso/mios/boot.cfg
	@echo "mode=debug" > iso/mios/debug.cfg
	@echo "vesa=true" >> iso/mios/debug.cfg
	@echo "verbose=true" >> iso/mios/debug.cfg
	@echo "mode=console" > iso/mios/console.cfg
	@echo "vesa=false" >> iso/mios/console.cfg
	@echo 'set timeout=5' > iso/boot/grub/grub.cfg
	@echo 'set timeout_style=menu' >> iso/boot/grub/grub.cfg
	@echo 'set default=0' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'insmod png' >> iso/boot/grub/grub.cfg
	@echo 'insmod all_video' >> iso/boot/grub/grub.cfg
	@echo 'insmod vbe' >> iso/boot/grub/grub.cfg
	@echo 'insmod gfxterm' >> iso/boot/grub/grub.cfg
	@echo 'insmod gfxmenu' >> iso/boot/grub/grub.cfg
	@echo 'insmod font' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'set gfxmode=1024x768x32' >> iso/boot/grub/grub.cfg
	@echo 'set gfxpayload=keep' >> iso/boot/grub/grub.cfg
	@echo 'terminal_output gfxterm' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'background_image /boot/grub/background.png' >> iso/boot/grub/grub.cfg
	@echo 'set color_normal=light-gray/black' >> iso/boot/grub/grub.cfg
	@echo 'set color_highlight=white/blue' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "MiOS" {' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/kernel.bin' >> iso/boot/grub/grub.cfg
	@echo '    module2 /boot/initrd.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /mios/boot.cfg' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	grub-mkrescue -o $@ iso 2>/dev/null
	@echo "  ISO created: $@"

# Запуск с BIOS CD
run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -m 1024M -vga $(VGA) -no-reboot -no-shutdown

# Очистка
clean:
	cargo clean
	rm -rf target iso $(ISO) $(INITRD) initrd
	@echo "  Cleaned"

# Настройка
setup:
	rustup override set nightly
	rustup component add rust-src
	rustup target add $(TARGET)
	@echo "  Setup done"
	@echo ""
	@echo "  Установите зависимости:"
	@echo "    sudo apt-get update"
	@echo "    sudo apt-get install grub-pc-bin xorriso"