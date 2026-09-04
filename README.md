# Harbour ↔ Ratatui for Windows

A working minimal binding between Harbour and
[Ratatui 0.30.2](https://crates.io/crates/ratatui/0.30.2), tested with:

- Harbour + Zig 64 (`-comp=zig -cpu=x86_64`);
- Harbour + MinGW-w64 64 (`-comp=mingw64 -cpu=x86_64`);
- Harbour + MinGW 32 (`-comp=mingw -cpu=x86`).

The three demos show the same Ratatui dashboard with a layout, `Block`,
`Paragraph`, `Gauge`, colors, and Unicode borders.

There is also an interactive showcase demo with RGB TrueColor, drop-down menus,
a hierarchical tree with `☑`/`☐`, stateful `List` and `Table` widgets, animated
`Gauge`, `Braille Chart`, `Sparkline`, `BarChart`, and multilingual Unicode text.
The bottom `Rich features` panel is generated in Harbour and displays a smooth
24-bit RGB spectrum, color capabilities, and actual text modifiers: bold, dim,
italic, underline, strikethrough, reverse, and blink.
`RTUI_PRESENT()` sends the frame to the same Windows console handle through
`WriteConsoleW` with VT mode enabled, so ANSI commands are interpreted instead
of being printed as text.

In the showcase, `F6` switches focus between the tree and the table. `↑`/`↓`
move the row only in the active panel; `Space` toggles a checkbox only in the
tree. `+` expands and `-` collapses the selected `Toolchains` or `Widgets`
group. `←`/`→` open and switch the main menu.

```powershell
.\demos\showcase\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-zig64 `
  -CompilerBin C:\zig `
  -Run
```

Within the demo, `↑`/`↓` change the selected row, `Space` toggles a checkbox,
`P` pauses/resumes the animation, and `Q` or `Esc` closes the application.

## Ready-to-run binaries

The ready-to-run Windows demos are in [`dist/`](dist/):

- `dist/showcase/showcase.exe` + `harbour_ratatui.dll` — interactive demo;
- `dist/zig64/demo_zig64.exe` + `harbour_ratatui.dll`;
- `dist/mingw64/demo_mingw64.exe` + `harbour_ratatui.dll`;
- `dist/mingw32/demo_mingw32.exe` + `harbour_ratatui.dll`.

Run each `.exe` from its own directory so that the matching DLL is next to it.
The binaries are provided only for a quick start; the build scripts remain
available for rebuilding from source.

## How it is connected

```text
Harbour .prg
    │  RTUI_FRAME_*() → RTUI_RENDER_COMMANDS()
    ▼
harbour/hb_ratatui.c       Harbour Extend API + LoadLibrary
    │  versioned C ABI
    ▼
harbour_ratatui.dll        Rust cdylib
    │
    ▼
Ratatui 0.30.2             off-screen Buffer → UTF-8/ANSI
```

New code uses the generic `HRC1` command-buffer interface. The UI layout, data,
RGB styles, and widget state are described in Harbour through
[`harbour/ratatui_builder.prg`](harbour/ratatui_builder.prg). The Rust adapter
only validates and executes the commands. The `RTUI_SHOWCASE*` functions remain
only for backward compatibility and are not needed for new screens.

The DLL is loaded dynamically. Therefore, one x64 DLL works with both Zig64 and
MinGW64, regardless of the format of their import libraries. MinGW32 loads a
separate i686 DLL. Memory is not freed across a foreign runtime boundary:
Harbour supplies a caller-owned buffer, and Rust only fills it.

## Requirements

1. Windows 10 or later for ANSI colors.
2. A current stable Rust toolchain (`rustup`, `cargo`, `rustc`).
3. MSVC Build Tools, required by the Rust linker for a `*-pc-windows-msvc` DLL.
4. A Harbour package for the target architecture.
5. A Zig or MinGW C compiler matching the Harbour package.

The MinGW32 and MinGW64 Harbour packages are available from
[FiveTechSoft Harbour Builder](https://fivetechsoft.github.io/Harbour_builder/).
Extract them into separate directories, for example:

```text
C:\toolchains\harbour-mingw64
C:\toolchains\harbour-mingw32
```

Do not mix the 32-bit and 64-bit `bin`, `include`, and `lib` directories.

Zig64 is not a separate download on that page. It requires Harbour built with
compiler ID `zig` and a 64-bit Zig executable either in `PATH` or supplied with
`-CompilerBin`.

## Building and running

Run all commands from the repository root in PowerShell. You may omit
`-CompilerBin` if `zig.exe`/`gcc.exe` is already in `PATH`.

### Zig64

```powershell
.\demos\zig64\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-zig64 `
  -CompilerBin C:\zig `
  -Run
```

This produces:

```text
demos\zig64\demo_zig64.exe
demos\zig64\harbour_ratatui.dll
```

### MinGW64

```powershell
.\demos\mingw64\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-mingw64 `
  -CompilerBin C:\toolchains\mingw64\bin `
  -Run
```

This produces:

```text
demos\mingw64\demo_mingw64.exe
demos\mingw64\harbour_ratatui.dll
```

### MinGW32

Add the 32-bit Rust standard library once:

```powershell
rustup target add i686-pc-windows-msvc
```

Then run:

```powershell
.\demos\mingw32\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-mingw32 `
  -CompilerBin C:\toolchains\mingw32\bin `
  -Run
```

This produces:

```text
demos\mingw32\demo_mingw32.exe
demos\mingw32\harbour_ratatui.dll
```

When recompiling only the Harbour side, you can skip the Cargo build:

```powershell
.\demos\zig64\build.ps1 -HarbourRoot C:\toolchains\harbour-zig64 -SkipRust
```

## Using it in your own Harbour project

Add the C bridge file to your `.hbp`:

```text
myapp.prg
path/to/harbour_ratatui/harbour/hb_ratatui.c
```

Copy the appropriate `harbour_ratatui.dll` next to `myapp.exe` and call:

```harbour
PROCEDURE Main()
   LOCAL cView

   IF ! RTUI_AVAILABLE()
      ? RTUI_LAST_ERROR()
      RETURN
   ENDIF

   cView := RTUI_RENDER( "My application", ;
      "This text is laid out and rendered by Ratatui.", ;
      72, 15, .T. )

   IF cView == NIL
      ? RTUI_LAST_ERROR()
   ELSEIF ! RTUI_PRESENT( cView, .T. )
      ? RTUI_LAST_ERROR()
   ENDIF
RETURN
```

The public Harbour API is:

- `RTUI_AVAILABLE()` — loads the DLL and checks the ABI;
- `RTUI_ABI_VERSION()` — returns the current ABI version (`1`);
- `RTUI_RENDER(cTitle, cBody, nWidth, nHeight, lAnsi)` — returns a rendered
  UTF-8 string or `NIL`;
- `RTUI_RENDER_COMMANDS(cBinary, lAnsi)` — executes a generic `HRC1` frame
  created by the Harbour builder;
- `RTUI_SHOWCASE(nTick, nSelected, nWidth, nHeight, lAnsi)` — renders the basic
  dashboard;
- `RTUI_SHOWCASE_EX(nTick, nSelected, nMenu, nMenuItem, nCheckedMask,`
  `lMenuOpen, nWidth, nHeight, lAnsi)` — renders the interactive dashboard;
  Harbour manages the menu, selected row, and checkboxes;
- `RTUI_SHOWCASE_UI(nTick, nTreeSelected, nTableSelected, nFocus, nMenu,`
  `nMenuItem, nCheckedMask, lMenuOpen, nWidth, nHeight, lAnsi)` — adds
  independent focus and selection for the tree and table;
- `RTUI_SHOWCASE_TREE(nTick, nTreeSelected, nTableSelected, nFocus, nMenu,`
  `nMenuItem, nCheckedMask, nExpandedMask, lMenuOpen, nWidth, nHeight, lAnsi)`
  — adds expansion and collapsing of hierarchical groups;
- `RTUI_PRESENT(cUtf8, lAnsi)` — outputs the UTF-8 frame through native
  `MultiByteToWideChar` + `WriteConsoleW`; `.T.` enables Windows VT for 24-bit
  RGB, while `.F.` rejects a string containing an ESC byte;
- `RTUI_LAST_ERROR()` — returns the most recent loader/Rust error;
- `RTUI_ENABLE_VT()` — enables ANSI processing for Windows console output.

If the DLL is not next to the executable, set its absolute path before launch:

```powershell
$env:HB_RATATUI_DLL = 'D:\myapp\bin\harbour_ratatui.dll'
.\myapp.exe
```

`RTUI_RENDER()` accepts widths from 24 to 500 and heights from 9 to 200. All
strings crossing the ABI are UTF-8 with an explicit byte length; they are not
NUL-terminated parameters.

### Text modifiers

`RTUI_FRAME_PARAGRAPH()` accepts an optional final `nModifiers` parameter. The
`RTUI_MOD_BOLD`, `RTUI_MOD_DIM`, `RTUI_MOD_ITALIC`, `RTUI_MOD_UNDERLINE`,
`RTUI_MOD_BLINK`, `RTUI_MOD_REVERSE`, `RTUI_MOD_CROSSED`, and
`RTUI_MOD_RAPID_BLINK` masks are combined by addition:

```harbour
#include "ratatui.ch"

RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 1, 1, 40, 3 ), "", ;
   "Bold + italic + underline", aWhite, aBackground, aBorder, ;
   0, .F., .F., .F., ;
   RTUI_MOD_BOLD + RTUI_MOD_ITALIC + RTUI_MOD_UNDERLINE )
```

The old `lBold` parameter remains valid and is used when `nModifiers` is not
provided.

## Direct build commands

The scripts execute the equivalent of:

```powershell
cargo build --release
cd demos\zig64
C:\toolchains\harbour-zig64\bin\hbmk2.exe demo_zig64.hbp -comp=zig -cpu=x86_64 -odemo_zig64
```

For MinGW64, change the profile to `-comp=mingw64 -cpu=x86_64`. For MinGW32,
use `-comp=mingw -cpu=x86` and the i686 DLL from
`target\i686-pc-windows-msvc\release`.

## Checks

```powershell
cargo fmt --check
cargo test
cargo build --release
```

The Rust exports are described in `include/harbour_ratatui.h`. The ABI is
protected by a version check, every Rust panic is caught at the FFI boundary,
and no Rust pointers or Rust-owned allocations are passed to Harbour.
