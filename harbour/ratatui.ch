/* Public Harbour API exported by hb_ratatui.c.
 *
 * RTUI_AVAILABLE()                         -> logical
 * RTUI_ABI_VERSION()                       -> numeric
 * RTUI_LAST_ERROR()                        -> UTF-8 string
 * RTUI_RENDER(title, body, w, h, ansi)     -> UTF-8/ANSI string or NIL
 * RTUI_RENDER_COMMANDS(binary, ansi)        -> generic command frame or NIL
 * RTUI_SHOWCASE(tick, selected, w, h, ansi)-> UTF-8/ANSI string or NIL
 * RTUI_SHOWCASE_EX(tick, selected, menu, menuItem, checkedMask,
 *                  menuOpen, w, h, ansi)   -> interactive showcase frame
 * RTUI_SHOWCASE_UI(tick, treeSelected, tableSelected, focus, menu,
 *                  menuItem, checkedMask, menuOpen, w, h, ansi)
 *                                            -> independently focused panels
 * RTUI_SHOWCASE_TREE(tick, treeSelected, tableSelected, focus, menu,
 *                  menuItem, checkedMask, expandedMask, menuOpen, w, h, ansi)
 *                                            -> collapsible tree groups
 * RTUI_PRESENT(utf8, ansi)                 -> native UTF-16/VT console output
 * RTUI_ENABLE_VT()                         -> logical
 */

#define HRUI_ABI_VERSION  1

/* Optional style mask for RTUI_FRAME_PARAGRAPH(..., nModifiers). */
#define RTUI_MOD_NONE          0
#define RTUI_MOD_BOLD          1
#define RTUI_MOD_DIM           2
#define RTUI_MOD_ITALIC        4
#define RTUI_MOD_UNDERLINE     8
#define RTUI_MOD_BLINK         16
#define RTUI_MOD_REVERSE       32
#define RTUI_MOD_CROSSED       64
#define RTUI_MOD_RAPID_BLINK   128
#define RTUI_MOD_MASK_MAX      255
#define RTUI_MOD_UNDERLINED    RTUI_MOD_UNDERLINE
#define RTUI_MOD_REVERSED      RTUI_MOD_REVERSE
#define RTUI_MOD_CROSSED_OUT   RTUI_MOD_CROSSED
#define RTUI_MOD_STRIKETHROUGH RTUI_MOD_CROSSED
