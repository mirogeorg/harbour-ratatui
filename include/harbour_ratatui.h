#ifndef HARBOUR_RATATUI_H_
#define HARBOUR_RATATUI_H_

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#  define HRUI_EXPORT __declspec(dllimport)
#else
#  define HRUI_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define HRUI_ABI_VERSION 1u
#define HRUI_OK 0
#define HRUI_INVALID_ARGUMENT (-1)
#define HRUI_BUFFER_TOO_SMALL (-2)
#define HRUI_PANIC (-3)

HRUI_EXPORT uint32_t hrui_abi_version(void);

HRUI_EXPORT int32_t hrui_render_dashboard(
    const uint8_t *title,
    size_t title_length,
    const uint8_t *body,
    size_t body_length,
    uint16_t width,
    uint16_t height,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT int32_t hrui_render_showcase(
    uint32_t tick,
    size_t selected,
    uint16_t width,
    uint16_t height,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT int32_t hrui_render_showcase_v2(
    uint32_t tick,
    size_t selected,
    size_t menu,
    size_t menu_item,
    uint32_t checked_mask,
    uint8_t menu_open,
    uint16_t width,
    uint16_t height,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT int32_t hrui_render_showcase_v3(
    uint32_t tick,
    size_t tree_selected,
    size_t table_selected,
    size_t focus,
    size_t menu,
    size_t menu_item,
    uint32_t checked_mask,
    uint8_t menu_open,
    uint16_t width,
    uint16_t height,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT int32_t hrui_render_showcase_v4(
    uint32_t tick,
    size_t tree_selected,
    size_t table_selected,
    size_t focus,
    size_t menu,
    size_t menu_item,
    uint32_t checked_mask,
    uint32_t expanded_mask,
    uint8_t menu_open,
    uint16_t width,
    uint16_t height,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT int32_t hrui_render_commands(
    const uint8_t *commands,
    size_t commands_length,
    uint8_t ansi,
    uint8_t *output,
    size_t output_capacity,
    size_t *output_length);

HRUI_EXPORT size_t hrui_last_error(char *output, size_t output_capacity);

#ifdef __cplusplus
}
#endif

#endif
