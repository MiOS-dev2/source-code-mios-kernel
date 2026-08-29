TARGET = x86_64-unknown-none
KERNEL = target/mios.elf
ISO = MiOS.iso
INITRD = initrd.tar
CARGO = cargo +nightly
VGA ?= vmware

# Выберите тему: tela, stylish, vimix, fluent
GRUB_THEME ?= stylish

LLD := $(shell which ld.lld 2>/dev/null || echo $(HOME)/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/ld.lld)

.PHONY: all clean run setup grub-theme

all: $(ISO)

# Создание initrd для основной ОС
$(INITRD):
	@mkdir -p initrd
	@echo "MiOS 5" > initrd/config.txt
	@echo "Welcome!" > initrd/readme.txt
	@echo "mode=normal" > initrd/boot.cfg
	@cd initrd && tar -cf ../$(INITRD) * 2>/dev/null || true
	@echo "  InitRD created"

# Компиляция загрузчика
target/boot.o: src/boot.asm
	@mkdir -p target
	nasm -f elf64 $< -o $@
	@echo "  Bootloader compiled"

# Компиляция основной ОС
target/$(TARGET)/release/libmios.a: FORCE
	RUSTFLAGS="-C link-arg=-nostdlib -C relocation-model=static -C code-model=kernel -C no-redzone=yes" \
	$(CARGO) build --release \
		-Z build-std=core,compiler_builtins \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET)
	@echo "  Rust kernel compiled"

FORCE:

# Линковка основной ОС
$(KERNEL): target/boot.o target/$(TARGET)/release/libmios.a linker.ld
	$(LLD) -T linker.ld -o $@ \
		target/boot.o \
		target/$(TARGET)/release/libmios.a \
		--gc-sections \
		-z max-page-size=0x1000
	@echo "  Kernel linked: $(KERNEL)"

# Копирование GRUB темы
grub-theme:
	@mkdir -p iso/boot/grub/themes
	@echo "  Installing GRUB theme: $(GRUB_THEME)..."
	@if [ -d /usr/share/grub/themes/$(GRUB_THEME) ]; then \
		cp -r /usr/share/grub/themes/$(GRUB_THEME) iso/boot/grub/themes/; \
		echo "  Theme $(GRUB_THEME) copied successfully!"; \
	else \
		echo "  Warning: Theme $(GRUB_THEME) not found! Creating fallback..."; \
		mkdir -p iso/boot/grub/themes/fallback; \
		echo "desktop-image: \"background.png\"" > iso/boot/grub/themes/fallback/theme.txt; \
		echo "desktop-color: \"#0a0a1a\"" >> iso/boot/grub/themes/fallback/theme.txt; \
		echo "item-color: \"#aaaaaa\"" >> iso/boot/grub/themes/fallback/theme.txt; \
		echo "selected-item-color: \"#ffffff\"" >> iso/boot/grub/themes/fallback/theme.txt; \
		echo "selected-item-bg-color: \"#0055aa\"" >> iso/boot/grub/themes/fallback/theme.txt; \
		convert -size 1024x768 gradient:\"#0a0a2e\"-\"#000000\" \
			-fill white -font Helvetica -pointsize 64 -weight bold \
			-annotate +150+200 "KACTOMIB3AUMB" \
			-fill cyan -font Helvetica -pointsize 32 \
			-annotate +150+270 "GRUB v6.2" \
			iso/boot/grub/themes/fallback/background.png 2>/dev/null || true; \
		echo "  Fallback theme created!"; \
	fi

# BIOS ISO с VBE и темой
$(ISO): $(KERNEL) $(INITRD) grub-theme
	@mkdir -p iso/boot/grub
	@mkdir -p iso/mios
	@mkdir -p iso/boot/grub/fonts
	
	cp $(KERNEL) iso/boot/mios.elf
	cp $(INITRD) iso/boot/initrd.tar
	
	# Копируем шрифты GRUB
	@if [ -f /usr/share/grub/unicode.pf2 ]; then \
		cp /usr/share/grub/unicode.pf2 iso/boot/grub/fonts/; \
	fi
	
	@if [ -f background.png ]; then cp background.png iso/boot/grub/background.png; fi
	
	# Файлы конфигурации MiOS
	@echo "mode=normal" > iso/mios/boot.cfg
	@echo "vesa=true" >> iso/mios/boot.cfg
	@echo "mode=debug" > iso/mios/debug.cfg
	@echo "vesa=true" >> iso/mios/debug.cfg
	@echo "verbose=true" >> iso/mios/debug.cfg
	@echo "mode=console" > iso/mios/console.cfg
	@echo "vesa=false" >> iso/mios/console.cfg
	
	# GRUB конфиг с VBE и темой
	@echo '# KACTOMIB3AUMB GRUB Configuration' > iso/boot/grub/grub.cfg
	@echo 'set timeout=15' >> iso/boot/grub/grub.cfg
	@echo 'set timeout_style=menu' >> iso/boot/grub/grub.cfg
	@echo 'set default=0' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo '# Load video modules for VBE' >> iso/boot/grub/grub.cfg
	@echo 'insmod all_video' >> iso/boot/grub/grub.cfg
	@echo 'insmod vbe' >> iso/boot/grub/grub.cfg
	@echo 'insmod vga' >> iso/boot/grub/grub.cfg
	@echo 'insmod gfxterm' >> iso/boot/grub/grub.cfg
	@echo 'insmod gfxterm_background' >> iso/boot/grub/grub.cfg
	@echo 'insmod gfxterm_menu' >> iso/boot/grub/grub.cfg
	@echo 'insmod png' >> iso/boot/grub/grub.cfg
	@echo 'insmod jpeg' >> iso/boot/grub/grub.cfg
	@echo 'insmod font' >> iso/boot/grub/grub.cfg
	@echo 'insmod multiboot2' >> iso/boot/grub/grub.cfg
	@echo 'insmod module2' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo '# Load font and set VBE mode' >> iso/boot/grub/grub.cfg
	@echo 'if loadfont /boot/grub/fonts/unicode.pf2; then' >> iso/boot/grub/grub.cfg
	@echo '    set gfxmode=1024x768x32,1024x768x24,1024x768,800x600x32,800x600x24,800x600,auto' >> iso/boot/grub/grub.cfg
	@echo '    set gfxpayload=keep' >> iso/boot/grub/grub.cfg
	@echo '    terminal_output gfxterm' >> iso/boot/grub/grub.cfg
	@echo 'fi' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo '# Load theme' >> iso/boot/grub/grub.cfg
	@echo 'set theme=/boot/grub/themes/$(GRUB_THEME)/theme.txt' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo '# MiOS Boot Menu' >> iso/boot/grub/grub.cfg
	@echo 'echo "[BOOT] Welcome to GRUB!"' >> iso/boot/grub/grub.cfg
	@echo 'echo ""' >> iso/boot/grub/grub.cfg
	@echo 'echo ""' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Boot MiOS" --class mios {' >> iso/boot/grub/grub.cfg
	@echo '    echo "[OK]  Loading MiOS kernel..."' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/mios.elf' >> iso/boot/grub/grub.cfg
	@echo '    module2 /boot/initrd.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /mios/boot.cfg' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "About MiOS" --class about {' >> iso/boot/grub/grub.cfg
	@echo '    echo "  MiOS Kernel v5.8"' >> iso/boot/grub/grub.cfg
	@echo '    echo "  by Mikhail"' >> iso/boot/grub/grub.cfg
	@echo '    echo "  https://mios-system.vercel.app"' >> iso/boot/grub/grub.cfg
	@echo '    echo ""' >> iso/boot/grub/grub.cfg
	@echo '    echo "  Press any key to continue..."' >> iso/boot/grub/grub.cfg
	@echo '    sleep 5' >> iso/boot/grub/grub.cfg
	@echo '    boot' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Reboot" --class reboot {' >> iso/boot/grub/grub.cfg
	@echo '    reboot' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Shutdown" --class shutdown {' >> iso/boot/grub/grub.cfg
	@echo '    halt' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	
	# Создаем ISO
	@echo "  Creating ISO with grub-mkrescue..."
	grub-mkrescue -o $@ iso \
		--modules="biosdisk part_msdos part_gpt ext2 iso9660 udf" \
		--fonts="unicode" \
		--themes="$(GRUB_THEME)" || \
	grub-mkrescue -o $@ iso \
		--fonts="unicode" \
		--themes="$(GRUB_THEME)"
	
	@echo "  ISO created with $(GRUB_THEME) theme and VBE support: $@"

# Запуск с BIOS CD и VBE
run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -m 1024M -vga $(VGA) -no-reboot -no-shutdown

# Очистка
clean:
	cargo clean
	rm -rf target iso $(ISO) $(INITRD) initrd
	@echo "  Cleaned"

# Установка тем
install-themes:
	@echo "  Installing GRUB themes..."
	@if [ ! -d /usr/share/grub/themes/tela ]; then \
		cd /tmp && \
		git clone https://github.com/vinceliuice/grub2-themes.git && \
		cd grub2-themes && \
		sudo ./install.sh -t tela && \
		sudo ./install.sh -t stylish && \
		sudo ./install.sh -t vimix && \
		cd .. && rm -rf grub2-themes; \
		echo "  Themes installed!"; \
	else \
		echo "  Themes already installed!"; \
	fi

# Настройка
setup:
	rustup override set nightly
	rustup component add rust-src
	rustup target add $(TARGET)
	@echo "  Setup done"
	@echo ""
	@echo "  Install dependencies:"
	@echo "    sudo apt-get update"
	@echo "    sudo apt-get install grub-pc-bin xorriso nasm imagemagick"
	@echo "    sudo apt-get install fonts-dejavu fonts-dejavu-extra"
	@echo ""
	@echo "  To install themes:"
	@echo "    make install-themes"
	@echo ""
	@echo "  To build with different theme:"
	@echo "    make GRUB_THEME=stylish all"
	@echo "    make GRUB_THEME=vimix all"