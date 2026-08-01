INSTALLER_ISO = MiOS-Installer.iso

all: $(INSTALLER_ISO)

$(INSTALLER_ISO):
	@mkdir -p iso/boot/grub
	@mkdir -p iso/install
	@cp ../iso/boot/kernel.bin iso/boot/
	@cp ../iso/boot/initrd.tar iso/boot/
	@cp install.sh iso/install/
	@echo 'set timeout=10' > iso/boot/grub/grub.cfg
	@echo 'set timeout_style=menu' >> iso/boot/grub/grub.cfg
	@echo 'set default=0' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Install MiOS" {' >> iso/boot/grub/grub.cfg
	@echo '    multiboot2 /boot/kernel.bin' >> iso/boot/grub/grub.cfg
	@echo '    module2 /boot/initrd.tar' >> iso/boot/grub/grub.cfg
	@echo '    module2 /install/install.sh' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	@echo '' >> iso/boot/grub/grub.cfg
	@echo 'menuentry "Boot from Hard Disk" {' >> iso/boot/grub/grub.cfg
	@echo '    chainloader +1' >> iso/boot/grub/grub.cfg
	@echo '}' >> iso/boot/grub/grub.cfg
	grub-mkrescue -o $@ iso 2>/dev/null
	@echo "  Installer ISO created: $@"

clean:
	rm -rf iso $(INSTALLER_ISO)