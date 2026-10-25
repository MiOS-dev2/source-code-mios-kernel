# MiUX 7.0.0

Любительская операционная система для x86_64 с графическим интерфейсом, написанная с нуля: ядро на Rust, оконный интерфейс на C, загрузчик на NASM + GRUB 2.

Версия: **MiUX 7.0.0**
Автор: Mikhail, команда MDEVS

---

## Возможности

**Ядро и железо**
- Ядро на Rust (`no_std`), загрузка через GRUB 2 (multiboot2, режим VBE/VESA)
- Графика на линейном фреймбуфере, обновление экрана только изменённой области (`src/fastblit.rs`)
- Клавиатура и мышь (колесо мыши, кольцевой буфер событий)
- Диски: ATA, SATA, ATAPI
- Файловые системы: **FAT32** (чтение и запись), **EXT4** (просмотр), TAR для initrd
- Комманды для работы с Файлами в Терминале:
- cd <путь> - перемещение в нужную папку
- format <буква диска> /y - полное форматирование диска в FAT32
- ls , dir - показывают содержимое папки в каторой вы находитесь
- cat <имя файла.разширение> - создаёт файл с вашим разширением в вашей папке
- mount <буква диска> - простое монтирование диска в FAT32
- Пользователи: вход в систему, `useradd`, `userdel`, `passwd`

**Режим работы (MiUX)**
- После загрузки система стартует в **текстовом режиме**: консоль на весь экран, вход (`login:` / `Password:`) и вся работа через терминал, как в Linux без графической оболочки. Доступны все команды терминала.
- `gui /start --all` - включает графический режим: появляется курсор и одно окно Terminal на синем фоне (без значков рабочего стола и панели задач).
- Внутри графического режима: `gl /start` (3D-шестерёнки), `notepad /start` (Блокнот), а также остальные оконные команды (`task /start`, `calendar /start`, `html /start`, `browser`, `net`, `media`, `support`, `cre-tool`, `pc --info`).
- Эти оконные команды в текстовом режиме не работают: сначала нужно выполнить `gui /start --all`.
- `gui /stop` (или закрытие окна Terminal) возвращает в текстовый режим. `shutdown` выключает систему.
- PageUp / PageDown листают историю в текстовом режиме.
- Графический установщик убран, остался только стандартный (`MiUX Install` в меню GRUB).
- Чтобы вернуть старый рабочий стол внутри `gui /start --all`, в `src/lib.rs` поставьте `const BARE_DESKTOP: bool = false;`.

**Графический интерфейс** (включается командой `gui /start --all`)
- Оконный менеджер на C (`src/ui/`): рамки и тени, свернуть / развернуть / закрыть, панель задач, меню «Пуск», часы
- 4 темы: Aero, Basic, Classic, Ubuntu
- Раскладка EN/RU: кнопка на панели задач и `Alt+Shift`
- Рабочий стол с папками и ярлыками на файлы

**Приложения**
- Terminal, Notepad, Paint (сохранение в `.bmp`), File Manager
- Settings, Network, Browser
- Task Manager, Calendar, Clock
- Cre Tool (графический конструктор приложений), GL (3D-шестерёнки), HTML Viewer, Support Center, просмотр BMP

**Сеть и интернет**
- Проводная сеть: Intel e1000, Realtek RTL8139, RTL8169/8168/8111
- Стек TCP/IP на smoltcp: DHCP, DNS, TCP, UDP, ICMP
- HTTP и HTTPS (TLS 1.3), перенаправления, gzip, cookies
- Wi-Fi: общий слой `src/net/wifi.rs` и окно в Network; драйверов конкретных карт пока нет
- Сертификаты сайтов пока не проверяются

**Браузер** (`src/web/`)
- Собственный движок: парсер HTML, CSS (селекторы, каскад, `var()`, `@media`, `<link>`, `@import`), вёрстка (блоки, таблицы, flex, grid, списки, float)
- Картинки PNG, JPEG, BMP, формы (GET/POST), история, якоря
- Кодировки UTF-8 и windows-1251
- JavaScript не выполняется

**Установщик**
- Пункт «MiUX Install» в меню GRUB
- Ставит GRUB и ядро на FAT32-раздел, создаёт `system.cfg` рядом с ядром
- В установленной системе настройки (обои, тема, раскладка, ярлыки) читаются из `system.cfg` при каждой загрузке
- LiveCD работает без установки

---

## Структура проекта

```
miux_7/
├── src/
│   ├── lib.rs          главный цикл, окна, приложения, терминал
│   ├── boot.asm        загрузчик (переход в long mode, таблицы страниц, GDT)
│   ├── ui/             оконный интерфейс на C (wm.c, gfx.c, taskbar.c, ui.h)
│   ├── cui.rs, wm.rs   мост Rust <-> C и логика окон
│   ├── graphics.rs, vesa.rs, vga.rs, fastblit.rs   графика
│   ├── keyboard.rs, mouse.rs, idt.rs, timer.rs     ввод и прерывания
│   ├── ata.rs, sata.rs, atapi.rs, pci.rs           диски и PCI
│   ├── fs/             fat32.rs, ext4.rs, tar.rs
│   ├── net/            e1000, rtl8139, rtl8169, wifi, http
│   ├── web/            движок браузера
│   ├── installer.rs    установщик
│   ├── syscfg.rs       system.cfg: сохранение и загрузка настроек
│   └── fonts/, *.bmp   шрифты и значки
├── tools/gen_fonts.py  генератор сглаженных шрифтов
├── linker.ld, x86_64-miux.json, Cargo.toml, Makefile
└── initrd/             содержимое initrd
```

---

## Сборка

Нужны Linux (проверено на Ubuntu 24.04), nightly Rust, NASM, GRUB и QEMU.

```bash
sudo apt update
sudo apt install -y build-essential nasm grub-pc-bin grub-common xorriso mtools \
                    qemu-system-x86 fonts-dejavu fonts-dejavu-extra

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

make setup    # nightly, rust-src, target x86_64-unknown-none
make          # собирает MiUX.iso
```

Результат: `MiUX.iso`. Ядро линкуется в `target/miux.elf`.

## Запуск

```bash
make run
```

Или вручную, с сетью:

```bash
qemu-system-x86_64 -cdrom MiUX.iso -m 1024M -vga std -nic user,model=e1000
```

Можно взять `model=rtl8139`. Нужно не меньше 512 МБ памяти.

### Запуск с FAT32-диском

```bash
make fat32-disk   # создаёт fat32.img (100 МБ)
make run-fat32    # диск + ISO
```

Остальные цели: `run-dual`, `run-fat32-debug`, `run-fat32-gdb`, `fat32-disk-test`, `fat32-check`, `clean`, `help`.

### Меню GRUB

- **Boot MiUX** запускает систему
- **MiUX Install** запускает установщик
- **Reboot**, **Shutdown**

Параметры GRUB задаются только в `Makefile`, а `grub.cfg` создаётся при сборке. Загрузчик собирается с темой и фоном `background.png`.

---

## Установка на диск

1. Загрузитесь с ISO и выберите «MiUX Install».
2. Выберите диск.
3. Проверьте настройки и подтвердите установку.
4. Установщик копирует ядро в папку `MIUX`, ставит GRUB и создаёт `system.cfg`.
5. Загрузитесь с установленного диска.

---

# Полные ВСЕ необходиме зависимости для сборки:

- Обновление пакетов
- Обновление квартиры SUDO и обновление SUDO APT -y

## Основные инструменты сборки
- sudo apt install -y build-essential

## NASM (ассемблер для загрузчика)
- sudo apt install -y nasm

## GRUB и инструменты для создания ISO
- sudo apt install -y grub-pc-bin grub-common xorriso mtools

## Установка темы для GRUB
- Git Clone https://github.com/vinceliuice/grub2-themes.git CD Grub2-Themes sudo ./install.sh -t tela Судо ./install.sh -T Stylish sudo ./- - - install.sh -t vimix sudo update-grub

## QEMU для эмуляции
sudo apt install -y qemu-system-x86 qemu-kvm

## Зависимости для Rust (если ещё не установлен)
- curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | Тс

## После установки Rust перезагрузите терминал и выполните:
- Источник ~/.cargo/env

## Затем настройка Rust для проекта

## Команды терминала

Полный список: `help`.

| Группа | Команды |
|---|---|
| Система | `ver`, `uptime`, `mem`, `about`, `tech`, `kernel`, `pc --info` |
| Пользователи | `login`, `logout`, `whoami`, `users`, `useradd`, `userdel`, `passwd` |
| Файлы | `dir ls`, `fs`, `ata`, `tals`, `mount`, `format` |
| EXT4 | `ext4`, `ext4ls`, `ext4cat <файл>`, `ext4info` |
| Сеть | `ifconfig`, `nslookup <имя>`, `wget [-s] <адрес>`, `net` |
| Браузер | `browser [адрес]` |
| Пакеты | `pkg install <url>`: скачать файл и сохранить на диск |
| Приложения | `call <имя>`, `list-apps`, `cre-tool`, `media`, `html /start`, `support`, `gl /start`, `gl /stop` |
| Экран | `display /scr 1920x1080`, `display /scr default` |

Колесо мыши прокручивает вывод в терминале и страницы в браузере.

---

## Сборка шрифтов

Растры шрифтов лежат в `src/fonts/`. Чтобы пересоздать их, нужны `python3-pil` и шрифты DejaVu:

```bash
python3 tools/gen_fonts.py
```

---

## Известные ограничения

- JavaScript в браузере не выполняется
- Сертификаты HTTPS-сайтов не проверяются
- Нет драйверов конкретных Wi-Fi карт, в виртуальной машине Wi-Fi нет вообще
- RTL8169 на реальном железе не проверялся
- Ядро не включает SSE и FPU, поэтому C-код собирается с `-mgeneral-regs-only`

---

## Обратная связь

- Telegram: @mik_han3
- Gmail: questions2027@gmail.com
- Сайт: https://miux-system.vercel.app

*MDEVS (c) and Mikhail*
