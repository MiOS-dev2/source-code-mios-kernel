cat > install.sh << 'EOF'
#!/bin/bash

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MiOS Installer ===${NC}"
echo

# Проверка прав root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Ошибка: Запустите с sudo или от root${NC}"
    exit 1
fi

# Определение диска
echo -e "${YELLOW}Доступные диски:${NC}"
lsblk -d -o NAME,SIZE,MODEL

echo
read -p "Введите диск для установки (например, sda, sdb): " DISK

if [ -z "$DISK" ]; then
    echo -e "${RED}Диск не указан!${NC}"
    exit 1
fi

DISK_PATH="/dev/$DISK"
if [ ! -b "$DISK_PATH" ]; then
    echo -e "${RED}Диск $DISK_PATH не существует!${NC}"
    exit 1
fi

echo -e "${RED}ВНИМАНИЕ! Все данные на $DISK_PATH будут уничтожены!${NC}"
read -p "Продолжить? (y/N): " CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo "Установка отменена"
    exit 0
fi

# Создание разделов
echo -e "${GREEN}Создание разделов...${NC}"
parted -s $DISK_PATH mklabel msdos
parted -s $DISK_PATH mkpart primary 1MiB 100MiB
parted -s $DISK_PATH set 1 boot on
parted -s $DISK_PATH mkpart primary 100MiB 500MiB
parted -s $DISK_PATH mkpart primary 500MiB 100%

# Форматирование
echo -e "${GREEN}Форматирование разделов...${NC}"
mkfs.ext2 "${DISK_PATH}1"  # boot
mkfs.ext4 "${DISK_PATH}2"  # system
mkfs.ext4 "${DISK_PATH}3"  # data

# Монтирование
mkdir -p /mnt/mios
mount "${DISK_PATH}2" /mnt/mios
mkdir /mnt/mios/boot
mount "${DISK_PATH}1" /mnt/mios/boot

# Копирование файлов
echo -e "${GREEN}Копирование MiOS...${NC}"
mkdir -p /mnt/mios/boot/grub
mkdir -p /mnt/mios/mios
mkdir -p /mnt/mios/secure
mkdir -p /mnt/mios/gui

cp iso/boot/kernel.bin /mnt/mios/boot/
cp iso/boot/initrd.tar /mnt/mios/boot/
cp -r iso/mios/* /mnt/mios/mios/

# Копирование secure версии если есть
if [ -f iso/secure/kernel_secure.bin ]; then
    cp iso/secure/kernel_secure.bin /mnt/mios/boot/
    cp iso/secure/initrd_secure.tar /mnt/mios/boot/
fi

# Копирование GUI версии если есть
if [ -f iso/gui/kernel_gui.bin ]; then
    cp iso/gui/kernel_gui.bin /mnt/mios/boot/
    cp iso/gui/initrd_gui.tar /mnt/mios/boot/
fi

# Установка GRUB
echo -e "${GREEN}Установка GRUB...${NC}"
grub-install --target=i386-pc --boot-directory=/mnt/mios/boot $DISK_PATH

# Создание grub.cfg
echo -e "${GREEN}Создание конфигурации GRUB...${NC}"
cat > /mnt/mios/boot/grub/grub.cfg << 'GRUB_CFG'
set timeout=5
set timeout_style=menu
set default=0

menuentry "MiOS" {
    multiboot2 /boot/kernel.bin
    module2 /boot/initrd.tar
    module2 /mios/boot.cfg
}

menuentry "MiOS Secure" {
    multiboot2 /boot/kernel_secure.bin
    module2 /boot/initrd_secure.tar
    module2 /mios/boot.cfg
}

menuentry "MiOS GUI" {
    multiboot2 /boot/kernel_gui.bin
    module2 /boot/initrd_gui.tar
    module2 /mios/boot.cfg
}

menuentry "Mem Test" {
    echo "Running memory test..."
    sleep 3
    boot
}
GRUB_CFG

# Размонтирование
umount /mnt/mios/boot
umount /mnt/mios

echo -e "${GREEN}=== Установка завершена! ===${NC}"
echo -e "${YELLOW}Перезагрузите компьютер и загрузитесь с $DISK_PATH${NC}"
EOF

chmod +x install.sh