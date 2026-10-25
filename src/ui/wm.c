#include "ui.h"

#define UBUNTU (ui_t.id == 3)
#define CLASSIC (ui_t.id == 4)

void ui_draw_window_icon(int x, int y, int size) {
    int is = size * 2 / 3;
    int sx = x + (size - is) / 2, sy = y + (size - is) / 2;
    u32 hi = ui_rgb(200, 220, 240), lo = ui_rgb(100, 130, 170), fill = ui_rgb(180, 200, 230);
    for (int i = 0; i < is; i++) {
        ui_px(sx + i, sy, hi);
        ui_px(sx, sy + i, hi);
        ui_px(sx + is - 1, sy + i, lo);
        ui_px(sx + i, sy + is - 1, lo);
    }
    for (int dy = 1; dy < is - 1; dy++)
        for (int dx = 1; dx < is - 1; dx++)
            if (dx < is / 2 || dy < is / 2) ui_px(sx + dx, sy + dy, fill);
}

static void window_icon_ubuntu(int x, int y, int size) {
    int is = size * 2 / 3, r = 3;
    int sx = x + (size - is) / 2, sy = y + (size - is) / 2;
    ui_fill(sx, sy, is, is, ui_rgb(230, 230, 230));
    if (is > 2 * r) ui_fill(sx + r, sy + r, is - 2 * r, is - 2 * r, ui_rgb(200, 200, 200));
}

enum { SYM_MIN, SYM_MAX, SYM_CLOSE };

static void sym_cross(int cx, int cy, int size, u32 c) {
    int cs = size / 3;
    for (int i = 0; i < cs; i++) {
        ui_px(cx - cs / 2 + i, cy - cs / 2 + i, c);
        ui_px(cx - cs / 2 + i, cy + cs / 2 - i, c);
    }
}

static void win7_button(int x, int y, int size, int sym) {
    u32 bg = ui_rgb(220, 220, 230), border = ui_rgb(150, 150, 160), sc = ui_rgb(80, 80, 90);
    ui_fill(x, y, size, size, bg);
    ui_frame(x, y, size, size, border);
    int cx = x + size / 2, cy = y + size / 2;
    if (sym == SYM_MIN) {
        int lw = size / 2;
        ui_hline(cx - lw / 2, cy + 2, lw, sc);
    } else if (sym == SYM_MAX) {
        int b = size / 2;
        ui_frame(cx - b / 2, cy - b / 2, b, b, sc);
    } else {
        sym_cross(cx, cy, size, sc);
    }
    if (sym != SYM_CLOSE) ui_hline(x + 1, y + 1, size - 2, ui_rgb(245, 245, 250));
}

static void ubuntu_button(int x, int y, int size, int sym) {
    u32 c = ui_rgb(180, 180, 180);
    if (sym == SYM_CLOSE) {
        ui_fill(x, y, size, size, ui_rgb(48, 48, 48));
        sym_cross(x + size / 2, y + size / 2, size, c);
    } else if (sym == SYM_MIN) {
        ui_hline(x + size / 4, y + size / 2 + 2, size * 3 / 4 - size / 4, c);
    } else {
        int b = size / 3;
        ui_frame(x + size / 2 - b / 2, y + size / 2 - b / 2, b, b, c);
    }
}

static void classic_button(int x, int y, int size, int sym) {
    u32 face = ui_rgb(212, 208, 200), black = 0;
    ui_fill(x, y, size, size, face);
    ui_hline(x, y, size, 0xFFFFFF);
    ui_vline(x, y, size, 0xFFFFFF);
    ui_hline(x + 1, y + size - 2, size - 1, ui_rgb(128, 128, 128));
    ui_vline(x + size - 2, y + 1, size - 1, ui_rgb(128, 128, 128));
    ui_hline(x, y + size - 1, size, ui_rgb(64, 64, 64));
    ui_vline(x + size - 1, y, size, ui_rgb(64, 64, 64));
    int cx = x + size / 2, cy = y + size / 2;
    if (sym == SYM_MIN) {
        ui_fill(cx - 4, cy + 3, 7, 2, black);
    } else if (sym == SYM_MAX) {
        ui_frame(cx - 5, cy - 5, 10, 9, black);
        ui_hline(cx - 5, cy - 4, 10, black);
    } else {
        for (int i = 0; i < 8; i++) {
            ui_px(cx - 4 + i, cy - 4 + i, black);
            ui_px(cx - 3 + i, cy - 4 + i, black);
            ui_px(cx - 4 + i, cy + 3 - i, black);
            ui_px(cx - 3 + i, cy + 3 - i, black);
        }
    }
}

static void titlebar_classic(int x, int y, int w, int h, int active) {
    ui_fill(x, y, w, h, ui_rgb(212, 208, 200));
    int gx = x + 3, gy = y + 3, gw = w - 6, gh = h - 6;
    if (gw < 2 || gh < 1) return;
    u32 c0 = active ? ui_rgb(10, 36, 106) : ui_rgb(128, 128, 128);
    u32 c1 = active ? ui_rgb(166, 202, 240) : ui_rgb(192, 192, 192);
    for (int i = 0; i < gw; i++) ui_vline(gx + i, gy, gh, ui_mix(c0, c1, i * 256 / (gw - 1)));
}

typedef struct { int size, y, close_x, max_x, min_x; } buttons;

static buttons button_layout(const ui_window *w) {
    buttons b;
    b.size = UBUNTU ? 28 : 20;
    b.y = w->y + (UI_TITLEBAR_H - b.size) / 2;
    b.close_x = w->x + w->w - b.size - 8;
    b.max_x = b.close_x - b.size - 4;
    b.min_x = b.max_x - b.size - 4;
    return b;
}

static int in_rect(int px, int py, int x, int y, int w, int h) {
    return px >= x && px < x + w && py >= y && py < y + h;
}

static void titlebar_ubuntu(int x, int y, int w, int h, int active) {
    ui_fill(x, y, w, h, active ? ui_rgb(48, 48, 48) : ui_rgb(65, 65, 65));
    ui_hline(x, y + h - 1, w, active ? ui_rgb(60, 60, 60) : ui_rgb(75, 75, 75));
}

static void titlebar_glass(int x, int y, int w, int h, int active) {
    u32 top = active ? ui_rgb(160, 200, 245) : ui_rgb(190, 195, 205);
    u32 bot = active ? ui_rgb(80, 120, 180) : ui_rgb(140, 145, 155);
    ui_vgradient(x, y, w, h, top, bot, active ? 166 : 128);
    u32 glow = ui_rgb(240, 248, 255);
    for (int row = 0; row < 8 && row < h; row++) {
        int intensity = 120 - row * 100 / 8;
        int a = 64 + intensity * 256 / 200;
        for (int dx = 3; dx < w - 3; dx++) ui_px_alpha(x + dx, y + row, glow, a);
    }
    if (h > 2)
        for (int dx = 1; dx < w - 1; dx++) ui_px_alpha(x + dx, y + h - 1, 0xFFFFFF, 141);
}

void ui_draw_window(const ui_window *win) {
    if (win->state == UI_MINIMIZED) return;
    int x = win->x, y = win->y, w = win->w, h = win->h;
    if (w < 40 || h < UI_TITLEBAR_H + 4) return;
    int th = UI_TITLEBAR_H;

    if (!UBUNTU && !CLASSIC) {
        for (int k = 1; k <= 3; k++) {
            ui_hline(x + k + 2, y + h + k, w, 0);
            ui_vline(x + w + k + 2, y + k, h, 0);
        }
    }

    ui_fill(x, y + th, w, h - th, ui_t.window_bg);

    if (UBUNTU) titlebar_ubuntu(x, y, w, th, win->active);
    else if (CLASSIC) titlebar_classic(x, y, w, th, win->active);
    else titlebar_glass(x, y, w, th, win->active);

    buttons b = button_layout(win);
    int title_x = x + 36;
    int title_max = b.min_x - 8 - title_x;

    if (UBUNTU) {
        window_icon_ubuntu(x + 12, y + (th - 16) / 2, 16);
        u32 tc = win->active ? ui_rgb(230, 230, 230) : ui_rgb(185, 185, 185);
        ui_text_fit(title_x, y + 7, win->title, win->title_len, tc, tc, title_max);
        ubuntu_button(b.min_x, b.y, b.size, SYM_MIN);
        ubuntu_button(b.max_x, b.y, b.size, SYM_MAX);
        ubuntu_button(b.close_x, b.y, b.size, SYM_CLOSE);
        ui_hline(x, y + h - 1, w, ui_rgb(60, 60, 60));
        ui_vline(x + w - 1, y, h, ui_rgb(60, 60, 60));
    } else if (CLASSIC) {
        ui_draw_window_icon(x + 8, y + (th - 20) / 2, 20);
        u32 tc = win->active ? 0xFFFFFF : ui_rgb(212, 208, 200);
        ui_text_fit(title_x, y + 7, win->title, win->title_len, tc, tc, title_max);
        ui_text_fit(title_x + 1, y + 7, win->title, win->title_len, tc, tc, title_max);
        classic_button(b.min_x, b.y, b.size, SYM_MIN);
        classic_button(b.max_x, b.y, b.size, SYM_MAX);
        classic_button(b.close_x, b.y, b.size, SYM_CLOSE);
        ui_hline(x, y, w, ui_rgb(212, 208, 200));
        ui_vline(x, y, h, ui_rgb(212, 208, 200));
        ui_hline(x + 1, y + 1, w - 2, 0xFFFFFF);
        ui_vline(x + 1, y + 1, h - 2, 0xFFFFFF);
        ui_hline(x + 1, y + h - 2, w - 2, ui_rgb(128, 128, 128));
        ui_vline(x + w - 2, y + 1, h - 2, ui_rgb(128, 128, 128));
        ui_hline(x, y + h - 1, w, ui_rgb(64, 64, 64));
        ui_vline(x + w - 1, y, h, ui_rgb(64, 64, 64));
    } else {
        ui_draw_window_icon(x + 8, y + (th - 20) / 2, 20);
        static const int off[8][2] = {{-1,-1},{-1,0},{-1,1},{0,-1},{0,1},{1,-1},{1,0},{1,1}};
        for (int i = 0; i < 8; i++)
            ui_text_fit(title_x + off[i][0], y + 7 + off[i][1], win->title, win->title_len, 0, 0, title_max);
        u32 tc = win->active ? 0xFFFFFF : ui_rgb(200, 200, 210);
        ui_text_fit(title_x, y + 7, win->title, win->title_len, tc, tc, title_max);
        win7_button(b.min_x, b.y, b.size, SYM_MIN);
        win7_button(b.max_x, b.y, b.size, SYM_MAX);
        win7_button(b.close_x, b.y, b.size, SYM_CLOSE);
        ui_frame(x, y, w, h, ui_rgb(100, 100, 120));
        ui_frame(x + 1, y + 1, w - 2, h - 2, ui_rgb(200, 200, 210));
    }

    ui_fill(x + 2, y + th + 2, w - 4, h - th - 4, ui_t.window_bg);
}

int ui_window_hit(const ui_window *win, int px, int py) {
    if (win->state == UI_MINIMIZED) return UI_HIT_NONE;
    if (!in_rect(px, py, win->x, win->y, win->w, win->h)) return UI_HIT_NONE;
    buttons b = button_layout(win);
    if (in_rect(px, py, b.close_x, b.y, b.size, b.size)) return UI_HIT_CLOSE;
    if (in_rect(px, py, b.max_x, b.y, b.size, b.size)) return UI_HIT_MAX;
    if (in_rect(px, py, b.min_x, b.y, b.size, b.size)) return UI_HIT_MIN;
    if (py < win->y + UI_TITLEBAR_H) return UI_HIT_TITLE;
    if (px >= win->x + win->w - 15 && py >= win->y + win->h - 15) return UI_HIT_RESIZE;
    return UI_HIT_CLIENT;
}

int ui_zorder(const ui_window *wins, int n, int *order) {
    for (int i = 0; i < n; i++) {
        int j = i;
        while (j > 0 && wins[order[j - 1]].z > wins[i].z) { order[j] = order[j - 1]; j--; }
        order[j] = i;
    }
    return n;
}

int ui_top_at(const ui_window *wins, int n, int px, int py) {
    int best = -1;
    for (int i = 0; i < n; i++) {
        const ui_window *w = &wins[i];
        if (w->state == UI_MINIMIZED) continue;
        if (!in_rect(px, py, w->x, w->y, w->w, w->h)) continue;
        if (best < 0 || w->z > wins[best].z) best = i;
    }
    return best;
}
