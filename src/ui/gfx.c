#include "ui.h"

u32 *ui_bb;
int ui_w, ui_h;
ui_theme ui_t;

void ui_init(u32 *backbuffer, int width, int height) {
    ui_bb = backbuffer;
    ui_w = width;
    ui_h = height;
}

void ui_set_theme(const ui_theme *t) { ui_t = *t; }

u32 ui_rgb(int r, int g, int b) { return ((u32)r << 16) | ((u32)g << 8) | (u32)b; }

u32 ui_mix(u32 bg, u32 c, int a) {
    if (a >= 256) return c;
    if (a <= 0) return bg;
    int ia = 256 - a;
    u32 r = (((c >> 16) & 0xFF) * a + ((bg >> 16) & 0xFF) * ia) >> 8;
    u32 g = (((c >> 8) & 0xFF) * a + ((bg >> 8) & 0xFF) * ia) >> 8;
    u32 b = ((c & 0xFF) * a + (bg & 0xFF) * ia) >> 8;
    return (r << 16) | (g << 8) | b;
}

void ui_px(int x, int y, u32 c) {
    if (x < 0 || y < 0 || x >= ui_w || y >= ui_h) return;
    ui_bb[y * ui_w + x] = c;
}

u32 ui_get(int x, int y) {
    if (x < 0 || y < 0 || x >= ui_w || y >= ui_h) return 0;
    return ui_bb[y * ui_w + x];
}

void ui_px_alpha(int x, int y, u32 c, int a) {
    if (x < 0 || y < 0 || x >= ui_w || y >= ui_h) return;
    u32 *p = &ui_bb[y * ui_w + x];
    *p = ui_mix(*p, c, a);
}

static int clip(int *x, int *y, int *w, int *h) {
    if (*x < 0) { *w += *x; *x = 0; }
    if (*y < 0) { *h += *y; *y = 0; }
    if (*x + *w > ui_w) *w = ui_w - *x;
    if (*y + *h > ui_h) *h = ui_h - *y;
    return *w > 0 && *h > 0;
}

void ui_fill(int x, int y, int w, int h, u32 c) {
    if (!clip(&x, &y, &w, &h)) return;
    for (int j = 0; j < h; j++) {
        u32 *row = &ui_bb[(y + j) * ui_w + x];
        for (int i = 0; i < w; i++) row[i] = c;
    }
}

void ui_fill_alpha(int x, int y, int w, int h, u32 c, int alpha) {
    if (!clip(&x, &y, &w, &h)) return;
    int a = alpha + (alpha >> 7);
    for (int j = 0; j < h; j++) {
        u32 *row = &ui_bb[(y + j) * ui_w + x];
        for (int i = 0; i < w; i++) row[i] = ui_mix(row[i], c, a);
    }
}

void ui_hline(int x, int y, int w, u32 c) { ui_fill(x, y, w, 1, c); }
void ui_vline(int x, int y, int h, u32 c) { ui_fill(x, y, 1, h, c); }

void ui_frame(int x, int y, int w, int h, u32 c) {
    if (w <= 0 || h <= 0) return;
    ui_hline(x, y, w, c);
    ui_hline(x, y + h - 1, w, c);
    ui_vline(x, y, h, c);
    ui_vline(x + w - 1, y, h, c);
}

void ui_vgradient(int x, int y, int w, int h, u32 c0, u32 c1, int alpha) {
    for (int row = 0; row < h; row++) {
        int t = h > 1 ? row * 256 / (h - 1) : 0;
        u32 c = ui_mix(c0, c1, t);
        if (alpha >= 256) ui_fill(x, y + row, w, 1, c);
        else for (int i = 0; i < w; i++) ui_px_alpha(x + i, y + row, c, alpha);
    }
}

void ui_raised(int x, int y, int w, int h) {
    if (w <= 0 || h <= 0) return;
    u32 light = 0xFFFFFF, shadow = ui_rgb(80, 80, 90), dark = ui_rgb(40, 40, 45);
    ui_fill(x, y, w, h, ui_t.button_face);
    ui_hline(x, y, w, light);
    ui_vline(x, y + 1, h - 1, light);
    ui_vline(x + w - 1, y + 1, h - 1, shadow);
    ui_hline(x, y + h - 1, w, shadow);
    ui_px(x + w - 1, y + h - 1, dark);
    if (w >= 2 && h >= 2) { ui_px(x + w - 2, y + h - 1, dark); ui_px(x + w - 1, y + h - 2, dark); }
}

void ui_sunken(int x, int y, int w, int h) {
    if (w <= 0 || h <= 0) return;
    u32 light = 0xFFFFFF, shadow = ui_rgb(50, 50, 55), dark = ui_rgb(30, 30, 35);
    ui_fill(x, y, w, h, ui_t.button_face);
    ui_hline(x, y, w, shadow);
    ui_vline(x, y + 1, h - 1, shadow);
    ui_vline(x + w - 1, y + 1, h - 1, light);
    ui_hline(x, y + h - 1, w, light);
    ui_px(x, y, dark);
    if (w >= 2) ui_px(x + 1, y, dark);
    if (h >= 2) ui_px(x, y + 1, dark);
}

int ui_text_chars(const u8 *s, usize len) {
    int n = 0;
    for (usize i = 0; i < len; i++) if ((s[i] & 0xC0) != 0x80) n++;
    return n;
}

void ui_text(int x, int y, const char *s, u32 fg, u32 bg) {
    usize n = 0;
    while (s[n]) n++;
    if (x >= 0 && y >= 0) miux_ui_text(x, y, (const u8 *)s, n, fg, bg);
}

void ui_text_fit(int x, int y, const u8 *s, usize len, u32 fg, u32 bg, int max_px) {
    int maxc = max_px / 8;
    if (maxc <= 0 || x < 0 || y < 0) return;
    int n = ui_text_chars(s, len);
    if (n <= maxc) { miux_ui_text(x, y, s, len, fg, bg); return; }
    if (maxc <= 2) return;
    int keep = maxc - 2, c = 0;
    usize i = 0;
    for (; i < len; i++) {
        if ((s[i] & 0xC0) != 0x80) { if (c == keep) break; c++; }
    }
    miux_ui_text(x, y, s, i, fg, bg);
    miux_ui_text(x + keep * 8, y, (const u8 *)"..", 2, fg, bg);
}
