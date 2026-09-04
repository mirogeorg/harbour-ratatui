# HRC1 Harbour → Ratatui command protocol

`HRC1` is the stable seam between the Harbour application and the Rust adapter.
The application owns UI composition, state, and input handling. Rust validates
and executes generic Ratatui widget commands.

## Public Harbour interface

`harbour/ratatui_builder.prg` provides:

- `RTUI_FRAME_NEW()` and `RTUI_FRAME_RENDER()`;
- `RTUI_RGB()` and `RTUI_RECT()`;
- `RTUI_FRAME_CLEAR()`, `RTUI_FRAME_BLOCK()`, `RTUI_FRAME_PARAGRAPH()`;
- `RTUI_FRAME_TABS()`, `RTUI_FRAME_LIST()`, `RTUI_FRAME_TABLE()`;
- `RTUI_FRAME_GAUGE()`, `RTUI_FRAME_SPARKLINE()`;
- `RTUI_FRAME_BARCHART()` and `RTUI_FRAME_CHART()`.

Only `RTUI_RENDER_COMMANDS(cBinary, lAnsi)` and `RTUI_PRESENT()` cross the C
ABI. Changes to layouts, menus, tree state, tables, RGB styles, and data require
changes only to the Harbour code.

## Binary envelope

All numbers are little-endian. Strings are UTF-8 and are represented as
`u32 byte_length` + bytes.

```text
Header:
  byte[4] magic = "HRC1"
  u16     version = 1
  u16     frame_width
  u16     frame_height
  u16     command_count

Command:
  u8      opcode
  u8      flags (bit 0 = optional)
  u32     payload_length
  byte[]  payload
```

Version 1 opcodes:

| Opcode | Widget |
|---:|---|
| 1 | Clear |
| 2 | Block |
| 3 | Paragraph |
| 4 | Tabs |
| 5 | List |
| 6 | Gauge |
| 7 | Table |
| 8 | Sparkline |
| 9 | BarChart |
| 10 | Chart |

An unknown required command is an error. An unknown command with the optional
flag is skipped using its `payload_length`. The maximum buffer size is 8 MiB,
the maximum command count is 4096, and every widget rectangle must fit entirely
within the frame.

## Text modifiers from Harbour

The final byte in the `RTUI_FRAME_PARAGRAPH()` payload is a compact mask for
Ratatui's `Modifier`. The old `lBold` parameter remains compatible: when
`nModifiers` is not provided, `.T.` means `RTUI_MOD_BOLD`.

```harbour
#include "ratatui.ch"

RTUI_FRAME_PARAGRAPH( aFrame, aRect, "", "Bold italic", ;
   aWhite, aBackground, aBorder, 0, .F., .F., .F., ;
   RTUI_MOD_BOLD + RTUI_MOD_ITALIC )
```

The masks are:

| Constant | Value | ANSI SGR |
|---|---:|---:|
| `RTUI_MOD_BOLD` | 1 | 1 |
| `RTUI_MOD_DIM` | 2 | 2 |
| `RTUI_MOD_ITALIC` | 4 | 3 |
| `RTUI_MOD_UNDERLINE` | 8 | 4 |
| `RTUI_MOD_BLINK` | 16 | 5 (slow blink) |
| `RTUI_MOD_REVERSE` | 32 | 7 |
| `RTUI_MOD_CROSSED` | 64 | 9 |
| `RTUI_MOD_RAPID_BLINK` | 128 | 6 |

Combine masks by adding the constants. ANSI/VT supports blinking, but a
particular terminal may disable it or choose not to display it.

## Performance

The local Windows x64 benchmark from 2026-09-02 runs 2,000 frames in each
series, without input waits or console output:

```text
Command buffer: 0.894–0.942 ms/frame
Legacy hardcoded Rust showcase: 0.938–0.942 ms/frame
```

The result shows no measurable slowdown for the showcase workload. Start
benchmark mode with `HB_RATATUI_BENCHMARK=1`; set the number of frames through
`HB_RATATUI_BENCHMARK_ITERATIONS`.

After adding a 74-command TrueColor gradient and an additional 10-row panel
only in Harbour, the result is `1.208–1.257 ms/frame`. The extra visualization
costs about `0.3 ms/frame` and remains approximately 1.5% of the 80 ms showcase
tick.
