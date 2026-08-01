section .multiboot
align 8
multiboot_header:
    dd 0xE85250D6
    dd 0
    dd multiboot_header_end - multiboot_header
    dd -(0xE85250D6 + 0 + (multiboot_header_end - multiboot_header))

    align 8
    dw 5                         ; тег framebuffer
    dw 1                         ; флаги: линейный буфер обязательно
    dd 24
    dd 800
    dd 600
    dd 32

    align 8
    dw 0
    dw 0
    dd 8
multiboot_header_end:

section .boot
bits 32
global start
extern rust_main
extern _kernel_end

PML4_TABLE_ADDR   equ 0x10000
PDPT_TABLE_ADDR   equ 0x11000
PDT_TABLE_ADDR    equ 0x12000    ; PDT для 0–1 ГБ
PDT1_TABLE_ADDR   equ 0x14000    ; 1–2 ГБ
PDT2_TABLE_ADDR   equ 0x15000    ; 2–3 ГБ
PDT3_TABLE_ADDR   equ 0x16000    ; 3–4 ГБ
GDT64_ADDR        equ 0x13000

start:
    cli
    cld

    ; Сохраняем multiboot magic и info в свободную память (передадим в rust_main)
    mov dword [0x500], eax
    mov dword [0x504], ebx

    ; Проверяем Long Mode
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_lm

    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29          ; бит LM
    jz .no_lm

    ; Инициализация страничной структуры и GDT
    call init_paging
    call init_gdt64

    ; Загружаем GDT
    lgdt [gdt64_ptr]

    ; Указываем PML4 в CR3
    mov eax, PML4_TABLE_ADDR
    mov cr3, eax

    ; Включаем PAE (CR4.PAE = 1)
    mov eax, cr4
    or eax, 0x20
    mov cr4, eax

    ; Включаем Long Mode в EFER MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr

    ; Включаем paging (CR0.PG = 1). Теперь мы в 32-bit Compatibility mode
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax

    ; Far jump на 64-битный код (селектор 0x08 = первый 64-bit code segment)
    jmp 0x08:long_mode_start

.no_lm:
    ; Если LM не поддерживается
    mov dword [0xB8000], 0x4F4C4F4C
    hlt
    jmp $

; ------------ Инициализация страниц (2MB pages, покрывает 8 ГБ) ----------------
init_paging:
    pushad
    ; Чистим PML4, PDPT, PDT
    mov edi, PML4_TABLE_ADDR
    mov ecx, (4096 * 6) / 4    ; 24 КБ / 4
    xor eax, eax
    rep stosd

    ; 2. PML4[0] → PDPT (0x11000), R/W, Present
    mov eax, PDPT_TABLE_ADDR
    or eax, 0x03
    mov [PML4_TABLE_ADDR], eax

    ; 3. Заполняем PDPT: 4 записи, указывающие на 4 PDT
    mov eax, PDT_TABLE_ADDR
    or eax, 0x03
    mov [PDPT_TABLE_ADDR], eax          ; первый гигабайт

    mov eax, PDT1_TABLE_ADDR
    or eax, 0x03
    mov [PDPT_TABLE_ADDR + 8], eax      ; второй гигабайт

    mov eax, PDT2_TABLE_ADDR
    or eax, 0x03
    mov [PDPT_TABLE_ADDR + 16], eax     ; третий гигабайт

    mov eax, PDT3_TABLE_ADDR
    or eax, 0x03
    mov [PDPT_TABLE_ADDR + 24], eax     ; четвёртый гигабайт


    ; 4. Заполняем каждую PDT 512-ю 2-МБ страницами с флагами 0x83
    ; Макрос для заполнения PDT
    %macro fill_pdt 2   ; %1 = адрес PDT, %2 = начальный базовый адрес (для первой страницы)
        mov edi, %1
        mov ebx, %2
        mov ecx, 512
    %%loop:
        mov [edi], ebx
        add ebx, 0x200000
        add edi, 8
        loop %%loop
    %endmacro

    fill_pdt PDT_TABLE_ADDR,  0x00000083
    fill_pdt PDT1_TABLE_ADDR, 0x40000083   ; 1 ГБ = 0x40000000, первый 2-МБ блок: 0x40000000
    fill_pdt PDT2_TABLE_ADDR, 0x80000083   ; 2 ГБ
    fill_pdt PDT3_TABLE_ADDR, 0xC0000083   ; 3 ГБ

    popad
    ret

; ------------ GDT64 ---------------
init_gdt64:
    pushad
    ; Чистим область GDT (5 записей по 8 байт = 40 байт)
    mov edi, GDT64_ADDR
    mov ecx, 10               ; 40 байт / 4 = 10 dword'ов
    xor eax, eax
    rep stosd

    ; Запись 1: 64-bit Code Segment (DPL=0)
    mov dword [GDT64_ADDR + 8], 0x0000FFFF   ; limit[15:0] и base[15:0]
    mov dword [GDT64_ADDR + 12], 0x00209A00  ; present, dpl, б, тип (L=1, D=0) => 64bit code (0x9A)

    ; Запись 2: 64-bit Data Segment (DPL=0)
    mov dword [GDT64_ADDR + 16], 0x0000FFFF
    mov dword [GDT64_ADDR + 20], 0x00209200   ; 0x92 = present, dpl 0, data, r/w (0x93 – под вопросом, но 0x92 работает)

    ; (небольшое исправление: 0x00209200 – это 64-битный data? Для данных L-бит не имеет значения, главное D=0. Так что ок.)

    ; Запись 3: 64-bit Code Segment DPL=3 (пользовательский) – не нужна, оставим
    mov dword [GDT64_ADDR + 24], 0x0000FFFF
    mov dword [GDT64_ADDR + 28], 0x0020FA00   ; DPL=3 code

    ; Запись 4: 64-bit Data Segment DPL=3 (пользовательский) – не нужна
    mov dword [GDT64_ADDR + 32], 0x0000FFFF
    mov dword [GDT64_ADDR + 36], 0x0020F200   ; DPL=3 data

    ; Размер GDT и адрес
    mov word [gdt64_ptr], (5 * 8 - 1)
    mov dword [gdt64_ptr + 2], GDT64_ADDR
    popad
    ret

; -----------------------------------------------
bits 64
long_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov rsp, _kernel_end
    add rsp, 0x2000
    and rsp, ~0xF

    ; отладка: записать '!' в порт 0xE9
    mov dx, 0xE9
    mov al, '!'
    out dx, al

    mov edi, dword [0x500]
    mov esi, dword [0x504]
    call rust_main

    cli
    hlt
    jmp $

section .data
gdt64_ptr:
    dw 0
    dd 0
