#include "ui.h"

#define UBUNTU (ui_t.id == 3)

#define START_X 2
#define START_W 60
#define BTN_X0  68
#define BTN_W   100
#define BTN_GAP 4
#define LANG_W  36

static int tb_y(void) { return ui_h - UI_TASKBAR_H; }

static int clock_w(usize len) { return (int)len * 8 + 16; }
static int clock_x(usize len) { int cw = clock_w(len); return ui_w >= cw + 10 ? ui_w - cw - 5 : 0; }

static int lang_x(usize len) { int lx = clock_x(len) - BTN_GAP - LANG_W; return lx >= BTN_X0 ? lx : -1; }

static int button_x(int slot, usize clock_len) {
    int bx = BTN_X0 + slot * (BTN_W + BTN_GAP);
    int lx = lang_x(clock_len);
    if (bx + BTN_W > (lx >= 0 ? lx : clock_x(clock_len)) - BTN_GAP) return -1;
    return bx;
}

void ui_draw_taskbar(const ui_window *wins, int n, int menu_open, const u8 *clock, usize clock_len) {
    int y = tb_y();
    ui_fill(0, y, ui_w, UI_TASKBAR_H, ui_t.taskbar_bg);

    if (menu_open) ui_sunken(START_X, y + 2, START_W, 26);
    else ui_raised(START_X, y + 2, START_W, 26);
    miux_ui_text(START_X + (START_W - 40) / 2, y + 7, (const u8 *)"Start", 5, ui_t.button_text, ui_t.button_face);

    for (int i = 0; i < n; i++) {
        int bx = button_x(i, clock_len);
        if (bx < 0) break;
        if (wins[i].active && wins[i].state != UI_MINIMIZED) ui_sunken(bx, y + 3, BTN_W, 24);
        else ui_raised(bx, y + 3, BTN_W, 24);
        ui_text_fit(bx + 6, y + 7, wins[i].title, wins[i].title_len, ui_t.button_text, ui_t.button_face, BTN_W - 12);
    }

    int lx = lang_x(clock_len);
    if (lx >= 0) {
        ui_raised(lx, y + 3, LANG_W, 24);
        miux_ui_text(lx + (LANG_W - 16) / 2, y + 7, (const u8 *)(miux_ui_lang() ? "RU" : "EN"), 2, ui_t.button_text, ui_t.button_face);
    }

    int cx = clock_x(clock_len);
    ui_raised(cx, y + 3, clock_w(clock_len), 24);
    miux_ui_text(cx + 8, y + 7, clock, clock_len, ui_t.button_text, ui_t.button_face);
}

int ui_taskbar_hit(const ui_window *wins, int n, int px, int py, usize clock_len) {
    (void)wins;
    int y = tb_y();
    if (py < y || py >= ui_h) return UI_TB_NONE;
    if (px >= 0 && px < START_X + START_W) return UI_TB_START;
    int cx = clock_x(clock_len);
    if (px >= cx && px < cx + clock_w(clock_len)) return UI_TB_CLOCK;
    int lx = lang_x(clock_len);
    if (lx >= 0 && px >= lx && px < lx + LANG_W) return UI_TB_LANG;
    for (int i = 0; i < n; i++) {
        int bx = button_x(i, clock_len);
        if (bx < 0) break;
        if (px >= bx && px < bx + BTN_W) return i;
    }
    return UI_TB_EMPTY;
}

#define MENU_X 4
#define MENU_W 200
#define MENU_H 141
#define MENU_ITEMS 5

static const char *const menu_items[MENU_ITEMS] = { "Terminal", "Files", "Settings", "Calculator", "Shutdown" };

static int menu_y(void) { return tb_y() - MENU_H - 2; }

static int item_top(int i) { return menu_y() + (UBUNTU ? 34 + i * 20 : 8 + i * 26); }
static int item_h(void) { return UBUNTU ? 20 : 24; }

static void icon_play(int x, int y, u32 c) {
    for (int i = 0; i < 6; i++) ui_vline(x + 2 + i, y + 2 + i, 12 - 2 * i, c);
}
static void icon_folder(int x, int y, u32 c) {
    ui_fill(x + 1, y + 3, 5, 2, c);
    ui_fill(x + 1, y + 5, 12, 8, c);
}
static void icon_gear(int x, int y, u32 c, u32 bg) {
    ui_fill(x + 3, y + 3, 8, 8, c);
    ui_fill(x + 6, y + 1, 2, 12, c);
    ui_fill(x + 1, y + 6, 12, 2, c);
    ui_fill(x + 6, y + 6, 2, 2, bg);
}
static void icon_calc(int x, int y, u32 c, u32 bg) {
    ui_frame(x + 2, y + 1, 10, 13, c);
    ui_fill(x + 4, y + 3, 6, 2, c);
    ui_fill(x + 4, y + 7, 2, 2, c);
    ui_fill(x + 8, y + 7, 2, 2, c);
    ui_fill(x + 4, y + 10, 2, 2, c);
    ui_fill(x + 8, y + 10, 2, 2, c);
    (void)bg;
}
static void icon_power(int x, int y, u32 c) {
    ui_frame(x + 2, y + 3, 10, 10, c);
    ui_fill(x + 5, y + 1, 4, 3, ui_rgb(45, 45, 45));
    ui_vline(x + 7, y + 1, 6, c);
}

void ui_draw_start_menu(void) {
    int x = MENU_X, y = menu_y(), w = MENU_W, h = MENU_H;
    if (UBUNTU) {
        u32 bg = ui_rgb(45, 45, 45);
        ui_fill(x, y, w, h, bg);
        ui_frame(x, y, w, h, ui_rgb(70, 70, 70));
        ui_fill(x + 1, y + 1, w - 2, 27, ui_rgb(55, 55, 55));
        u32 hc = ui_rgb(200, 200, 200);
        miux_ui_text(x + 12, y + 6, (const u8 *)"Applications", 12, hc, hc);
        ui_hline(x + 8, y + 30, w - 16, ui_rgb(70, 70, 70));
        for (int i = 0; i < MENU_ITEMS; i++) {
            int iy = item_top(i) + 2;
            u32 c = i == 4 ? ui_rgb(220, 80, 80) : ui_rgb(220, 220, 220);
            if (i == 0) icon_play(x + 8, iy + 1, c);
            else if (i == 1) icon_folder(x + 8, iy + 1, c);
            else if (i == 2) icon_gear(x + 8, iy + 1, c, bg);
            else if (i == 3) icon_calc(x + 8, iy + 1, c, bg);
            else icon_power(x + 8, iy + 1, c);
            ui_text(x + 28, iy, menu_items[i], c, c);
            if (i == 1) ui_hline(x + 8, item_top(i) + item_h() - 1, w - 16, ui_rgb(60, 60, 60));
        }
    } else {
        ui_fill(x, y, w, h, ui_t.start_menu_bg);
        ui_frame(x, y, w, h, ui_rgb(80, 80, 90));
        for (int i = 0; i < MENU_ITEMS; i++)
            ui_text(x + 12, item_top(i) + (item_h() - 16) / 2, menu_items[i], ui_t.button_text, ui_t.start_menu_bg);
    }
}

int ui_start_menu_contains(int px, int py) {
    return px >= MENU_X && px < MENU_X + MENU_W && py >= menu_y() && py < menu_y() + MENU_H;
}

int ui_start_menu_item(int px, int py) {
    if (!ui_start_menu_contains(px, py)) return -1;
    for (int i = 0; i < MENU_ITEMS; i++) {
        int t = item_top(i);
        if (py >= t && py < t + item_h()) return i;
    }
    return -1;
}
