# HRC1 Harbour → Ratatui command protocol

`HRC1` е стабилният seam между Harbour приложението и Rust adapter-а.
Приложението притежава UI composition, state и input handling. Rust валидира
и изпълнява общи Ratatui widget команди.

## Публичен Harbour interface

`harbour/ratatui_builder.prg` предоставя:

- `RTUI_FRAME_NEW()` и `RTUI_FRAME_RENDER()`;
- `RTUI_RGB()` и `RTUI_RECT()`;
- `RTUI_FRAME_CLEAR()`, `RTUI_FRAME_BLOCK()`, `RTUI_FRAME_PARAGRAPH()`;
- `RTUI_FRAME_TABS()`, `RTUI_FRAME_LIST()`, `RTUI_FRAME_TABLE()`;
- `RTUI_FRAME_GAUGE()`, `RTUI_FRAME_SPARKLINE()`;
- `RTUI_FRAME_BARCHART()` и `RTUI_FRAME_CHART()`.

Само `RTUI_RENDER_COMMANDS(cBinary, lAnsi)` и `RTUI_PRESENT()` пресичат C ABI.
Промени в layout, менюта, tree state, таблици, RGB стилове и данни изискват
промяна само в Harbour кода.

## Binary envelope

Всички числа са little-endian. Низовете са UTF-8 и се представят като
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

Непозната задължителна команда е грешка. Непозната команда с optional flag
се прескача чрез `payload_length`. Максималният buffer е 8 MiB, максимумът е
4096 команди, а всеки widget rectangle трябва да е изцяло във frame-а.

## Text modifiers от Harbour

Последният byte от payload-а на `RTUI_FRAME_PARAGRAPH()` е компактна маска за
Ratatui `Modifier`. Старият параметър `lBold` остава съвместим: ако
`nModifiers` не е подаден, `.T.` означава `RTUI_MOD_BOLD`.

```harbour
#include "ratatui.ch"

RTUI_FRAME_PARAGRAPH( aFrame, aRect, "", "Bold italic", ;
   aWhite, aBackground, aBorder, 0, .F., .F., .F., ;
   RTUI_MOD_BOLD + RTUI_MOD_ITALIC )
```

Маските са:

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

Комбинират се със събиране на константите. Blink се поддържа от ANSI/VT, но
конкретният терминал може да го изключва или да не го визуализира.

## Performance

Локалният Windows x64 benchmark от 2026-09-02 изпълнява 2000 кадъра във всяка
серия, без input wait и console output:

```text
Command buffer: 0.894–0.942 ms/frame
Legacy hardcoded Rust showcase: 0.938–0.942 ms/frame
```

Резултатът не показва измеримо забавяне за showcase натоварването. Benchmark
режимът се стартира с `HB_RATATUI_BENCHMARK=1`; броят кадри се задава чрез
`HB_RATATUI_BENCHMARK_ITERATIONS`.

След добавяне само в Harbour на 74-команден TrueColor gradient и допълнителен
10-редов панел резултатът е `1.208–1.257 ms/frame`. Допълнителната визуализация
струва около `0.3 ms/frame` и остава приблизително 1.5% от 80 ms showcase tick.
