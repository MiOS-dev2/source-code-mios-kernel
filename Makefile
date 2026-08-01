TARGET = x86_64-unknown-none
KERNEL = target/kernel.bin
KERNEL_SECURE = target/kernel_secure.bin
KERNEL_GUI = target/kernel_gui.bin
ISO = MiOS.iso
INITRD = initrd.tar
INITRD_SECURE = initrd_secure.tar
INITRD_GUI = initrd_gui.tar
CARGO = cargo +nightly
VGA ?= vmware

LLD := $(shell which ld.lld 2>/dev/null || echo $(HOME)/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/ld.lld)

.PHONY: all clean run setup secure gui

all: $(ISO)

# Создание initrd для основной ОС
$(INITRD):
	@mkdir -p initrd
	@echo "MiOS 5" > initrd/config.txt
	@echo "Welcome!" > initrd/readme.txt
	@echo "mode=normal" > initrd/boot.cfg
	@cd initrd && tar -cf ../$(INITRD) * 2>/dev/null || true
	@echo "  InitRD created"

# Создание initrd для secure ОС
$(INITRD_SECURE):
	@mkdir -p initrd_secure
	@echo "MiOS Secure" > initrd_secure/config.txt
	@echo "Secure Mode Active!" > initrd_secure/readme.txt
	@echo "mode=secure" > initrd_secure/boot.cfg
	@echo "encryption=true" >> initrd_secure/boot.cfg
	@echo "secure_boot=active" >> initrd_secure/boot.cfg
	@cd initrd_secure && tar -cf ../$(INITRD_SECURE) * 2>/dev/null || true
	@echo "  Secure InitRD created"

# Создание initrd для GUI ОС
$(INITRD_GUI):
	@mkdir -p initrd_gui
	@echo "MiOS GUI" > initrd_gui/config.txt
	@echo "GUI Mode Active!" > initrd_gui/readme.txt
	@echo "mode=gui" > initrd_gui/boot.cfg
	@echo "graphics=true" >> initrd_gui/boot.cfg
	@echo "resolution=1024x768" >> initrd_gui/boot.cfg
	@cd initrd_gui && tar -cf ../$(INITRD_GUI) * 2>/dev/null || true
	@echo "  GUI InitRD created"

# Компиляция загрузчика
target/boot.o: src/boot.asm
	@mkdir -p target
	nasm -f elf64 $< -o $@
	@echo "  Bootloader compiled"

# Компиляция основной ОС
target/$(TARGET)/release/libmios.a: FORCE
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@echo "  Rust kernel compiled"

# Компиляция secure ОС из src_secure/lib.rs
target/$(TARGET)/release/libmios_secure.a: FORCE
	@echo "  Building secure kernel from src_secure/lib.rs..."
	@cp Cargo.toml Cargo.toml.bak
	@sed 's/path = "src\/lib.rs"/path = "src_secure\/lib.rs"/g' Cargo.toml > Cargo.toml.tmp
	@mv Cargo.toml.tmp Cargo.toml
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@mv Cargo.toml.bak Cargo.toml
	@cp target/$(TARGET)/release/libmios.a target/$(TARGET)/release/libmios_secure.a
	@echo "  Secure kernel compiled"

# Компиляция GUI ОС из src_gui/lib.rs
target/$(TARGET)/release/libmios_gui.a: FORCE
	@echo "  Building GUI kernel from src_gui/lib.rs..."
	@cp Cargo.toml Cargo.toml.bak
	@sed 's/path = "src\/lib.rs"/path = "src_gui\/lib.rs"/g' Cargo.toml > Cargo.toml.tmp
	@mv Cargo.toml.tmp Cargo.toml
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@mv Cargo.toml.bak Cargo.toml
	@cp target/$(TARGET)/release/libmios.a target/$(TARGET)/release/libmios_gui.a
	@echo "  GUI kernel compiled"

FORCE:

# Линковка основной ОС
$(KERNEL): target/boot.o target/$(TARGET)/release/libmios.a linker.ld
	$(LLD) -T linker.ld -o $@ \
		target/boot.o \
		target/$(TARGET)/release/libmios.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  Kernel linked"

# Линковка secure ОС
$(KERNEL_SECURE): target/boot.o target/$(TARGET)/release/libmios_secure.a linker_secure.ld
	@if [ ! -f target/$(TARGET)/release/libmios_secure.a ]; then \
		echo "  Building secure kernel first..."; \
		$(MAKE) target/$(TARGET)/release/libmios_secure.a; \
	fi
	$(LLD) -T linker_secure.ld -o $@ \
		target/boot.o \
		target/$(TARGET)/release/libmios_secure.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  Secure kernel linked"

# Линковка GUI ОС
$(KERNEL_GUI): target/boot.o target/$(TARGET)/release/libmios_gui.a linker_gui.ld
	@if [ ! -f target/$(TARGET)/release/libmios_gui.a ]; then \
		echo "  Building GUI kernel first..."; \
		$(MAKE) target/$(TARGET)/release/libmios_gui.a; \
	fi
	$(LLD) -T linker_gui.ld -o $@ \
		target/boot.o \
		target/$(TARGET)/release/libmios_gui.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  GUI kernel linked"

# BIOS ISO
$(ISO): $(KERNEL) $(KERNEL_SECURE) $(KERNEL_GUI) $(INITRD) $(INITRD_SECURE) $(INITRD_GUI)
	@mkdir -p iso/boot/grub
	@mkdir -p iso/mios
	@mkdir -p iso/secure
	@mkdir -p iso/secure/mios
	@mkdir -p iso/gui
	@mkdir -p iso/gui/mios
	cp $(KERNEL) iso/boot/kernel.bin
	cp $(KERNEL_SECURE) iso/secure/kernel_secure.bin
	cp $(KERNEL_GUI) iso/gui/kernel_gui.bin
	cp $(INITRD) iso/boot/initrd.tar
	cp $(INITRD_SECURE) iso/secure/initrd_secure.tar
	cp $(INITRD_GUI) iso/gui/initrd_gui.tar
	@if [ -f background.png ]; then cp background.png iso/boot/grub/background.png; fi
	@echo "mode=normal" > iso/mios/boot.cfg
	@echo "vesa=true" >> iso/mios/boot.cfg
	@echo "mode=debug" > iso/mios/debug.cfg
	@echo "vesa=true" >> iso/mios/debug.cfg
	@echo "verbose=true" >> iso/mios/debug.cfg
	@echo "mode=console" > iso/mios/console.cfg
	@echo "vesa=false" >> iso/mios/console.cfg
	@echo "mode=secure" > iso/secure/mios/boot.cfg
	@echo "vesa=true" >> iso/secure/mios/boot.cfg
	@echo "encryption=enabled" >> iso/secure/mios/boot.cfg
	@echo "secure_boot=active" >> iso/secure/mios/boot.cfg
	@echo "mode=gui" > iso/gui/mios/boot.cfg
	@echo "vesa=true" >> iso/gui/mios/boot.cfg
	@echo "graphics=enabled" >> iso/gui/mios/boot.cfg
	@echo "resolution=1024x768" >> iso/gui/mios/boot.cfg
	@echo 'set timeout=5' > iso/boot/grub/grub.cfg
	@echo 'set timeout_style=menu' >> iso/boot/grub/grub.cfg
	@echo 'set default=0' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "MiOS" {' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/kernel.bin' >> iso/boot/grub/grub.cfg
	@echo '    module2 /boot/initrd.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /mios/boot.cfg' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Secure Mode Load" {' >> iso/boot/grub/grub.cfg
	@echo '    echo "[BOOT] Loading secure OS..."' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /secure/kernel_secure.bin' >> iso/boot/grub/grub.cfg
	@echo '    module2 /secure/initrd_secure.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /secure/mios/boot.cfg' >> iso/boot/grub/grub.cfg
	@echo '    echo "[OK] Secure OS loaded!"' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "GUI Mode" {' >> iso/boot/grub/grub.cfg
	@echo '    echo "[BOOT] Loading GUI OS..."' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /gui/kernel_gui.bin' >> iso/boot/grub/grub.cfg
	@echo '    module2 /gui/initrd_gui.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /gui/mios/boot.cfg' >> iso/boot/grub/grub.cfg
	@echo '    echo "[BOOT] GUI OS loaded!"' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "About" {' >> iso/boot/grub/grub.cfg
	@echo '    echo "MiOS Kernel 5.8"' >> iso/boot/grub/grub.cfg
	@echo '    echo "by Mikhail"' >> iso/boot/grub/grub.cfg
	@echo '    sleep 3' >> iso/boot/grub/grub.cfg
	@echo '    boot' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	grub-mkrescue -o $@ iso 2>/dev/null
	@echo "  ISO created: $@"

# Запуск с BIOS CD
run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -m 1024M -vga $(VGA) -no-reboot -no-shutdown

# Очистка
clean:
	cargo clean
	rm -rf target iso $(ISO) $(INITRD) $(INITRD_SECURE) $(INITRD_GUI) initrd initrd_secure initrd_gui
	rm -f Cargo.toml.bak Cargo.toml.tmp
	@echo "  Cleaned"

# Настройка
setup:
	rustup override set nightly
	rustup component add rust-src
	rustup target add $(TARGET)
	@mkdir -p src_gui
	@echo "  Setup done"
	@echo ""
	@echo "  Теперь у тебя есть:"
	@echo "    src/lib.rs        - обычная версия"
	@echo "    src_secure/lib.rs - secure версия"
	@echo "    src_gui/lib.rs    - GUI версия"