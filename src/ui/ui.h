#ifndef MIUX_UI_H
#define MIUX_UI_H

typedef unsigned int u32;
typedef unsigned char u8;
typedef unsigned long usize;

typedef struct {
    u32 window_bg, button_face, button_text, taskbar_bg, start_menu_bg,
        taskbar_text, desktop_icon_text, border_light, border_dark;
    int id;
} ui_theme;

enum { UI_NORMAL = 0, UI_MINIMIZED = 1, UI_MAXIMIZED = 2 };

typedef struct {
    int x, y, w, h;
    const u8 *title;
    usize title_len;
    int state;
    int active;
    u32 z;
} ui_window;

enum { UI_HIT_NONE = 0, UI_HIT_CLIENT, UI_HIT_TITLE, UI_HIT_CLOSE, UI_HIT_MAX, UI_HIT_MIN, UI_HIT_RESIZE };

enum { UI_TB_NONE = -1, UI_TB_START = -2, UI_TB_CLOCK = -3, UI_TB_EMPTY = -4, UI_TB_LANG = -5 };

#define UI_TITLEBAR_H 30
#define UI_TASKBAR_H  30

void ui_init(u32 *backbuffer, int width, int height);
void ui_set_theme(const ui_theme *t);

void ui_fill(int x, int y, int w, int h, u32 c);
void ui_fill_alpha(int x, int y, int w, int h, u32 c, int alpha );
void ui_frame(int x, int y, int w, int h, u32 c);
void ui_raised(int x, int y, int w, int h);
void ui_sunken(int x, int y, int w, int h);

void ui_draw_window(const ui_window *w);
int  ui_window_hit(const ui_window *w, int px, int py);
int  ui_zorder(const ui_window *wins, int n, int *order);
int  ui_top_at(const ui_window *wins, int n, int px, int py);

void ui_draw_taskbar(const ui_window *wins, int n, int menu_open, const u8 *clock, usize clock_len);
int  ui_taskbar_hit(const ui_window *wins, int n, int px, int py, usize clock_len);
void ui_draw_start_menu(void);
int  ui_start_menu_contains(int px, int py);
int  ui_start_menu_item(int px, int py);

extern void miux_ui_text(int x, int y, const u8 *s, usize len, u32 fg, u32 bg);
extern int miux_ui_lang(void);

extern u32 *ui_bb;
extern int ui_w, ui_h;
extern ui_theme ui_t;
u32  ui_rgb(int r, int g, int b);
u32  ui_mix(u32 bg, u32 c, int a );
void ui_px(int x, int y, u32 c);
u32  ui_get(int x, int y);
void ui_px_alpha(int x, int y, u32 c, int a );
void ui_hline(int x, int y, int w, u32 c);
void ui_vline(int x, int y, int h, u32 c);
void ui_vgradient(int x, int y, int w, int h, u32 c0, u32 c1, int alpha );
int  ui_text_chars(const u8 *s, usize len);
void ui_text(int x, int y, const char *s, u32 fg, u32 bg);
void ui_text_fit(int x, int y, const u8 *s, usize len, u32 fg, u32 bg, int max_px);
void ui_draw_window_icon(int x, int y, int size);

#endif
