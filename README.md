__Исходный код ядра Mi-KRNL 6.0.0__

_1 - системные зависимости и настройка_
Перед тем как начать компиляцию Mi-Kernel необхлжимо выполнить эти комманды:
# Обновление пакетов
sudo apt update && sudo apt upgrade -y

# Основные инструменты сборки
sudo apt install -y build-essential

# NASM (ассемблер для загрузчика)
sudo apt install -y nasm

# GRUB и инструменты для создания ISO
sudo apt install -y grub-pc-bin grub-common xorriso mtools

# Установка темы для GRUB
git clone https://github.com/vinceliuice/grub2-themes.git
cd grub2-themes
sudo ./install.sh -t tela
sudo ./install.sh -t stylish
sudo ./install.sh -t vimix
sudo update-grub

# QEMU для эмуляции
sudo apt install -y qemu-system-x86 qemu-kvm

# Зависимости для Rust (если ещё не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# После установки Rust перезагрузите терминал и выполните:
source ~/.cargo/env

# Затем настройка Rust для проекта
make setup


_2 как работает ядро?_
__Главные функции__
есть несколько глобальных функций в ядре
1 - работа с базовой графикой

2 - работа с клавиатурой и мышью

3 - настройка через интерфейс ядра

Всем извесно что драйвера хранятся в отдельных файла .rs (Это и яжу понятно)

Драйвера находятся в папке src

_3 - GRUB_
Загрузчиком ядра выступает GNU/GRUB 2.12 с vbe mode

сами параметры GRUB ЕСТЬ ТОЛЬКО В makefile!!!

grub.cfg создаётся после компиляции

Так же есть красивая тема (в makefile есть 3 на выбор, по умолчанию стоит tela)

_4 - поддержка FAT32_
В ядре есть наработки fat32, __НО__ они не полные!!!

если вы хотите допилить fat32 то весь код fat32 есть в папке fs!

_5 - обратная связь с разработчиками_

__Обратная связь__ с коммандой разработчиков MDEVS 

# Обратная связь с Mikhail (Telegram): @mik_han3

# Обратная звязь по *GMAIL* : questions2027@gmail.com


*MDEVS (c) and Mikhail*

_Приятного исполдьзования!_