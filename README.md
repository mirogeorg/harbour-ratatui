# Harbour ↔ Ratatui за Windows

Работещ минимален binding между Harbour и
[Ratatui 0.30.2](https://crates.io/crates/ratatui/0.30.2), проверен с:

- Harbour + Zig 64 (`-comp=zig -cpu=x86_64`);
- Harbour + MinGW-w64 64 (`-comp=mingw64 -cpu=x86_64`);
- Harbour + MinGW 32 (`-comp=mingw -cpu=x86`).

Трите демота показват един и същ Ratatui dashboard с layout, `Block`,
`Paragraph`, `Gauge`, цветове и Unicode рамки.

Има и интерактивно showcase демо с RGB TrueColor, падащи менюта,
йерархично дърво с `☑`/`☐`, stateful `List` и `Table`, анимирани `Gauge`,
`Braille Chart`, `Sparkline`, `BarChart` и многоезичен Unicode текст.
Долният `Rich features` панел се генерира в Harbour и показва плавен 24-bit
RGB спектър, color capabilities и реални текстови модификатори: bold, dim,
italic, underline, strikethrough, reverse и blink.
`RTUI_PRESENT()` подава frame-а към същия Windows console handle чрез
`WriteConsoleW` и включен VT режим, така че ANSI командите се интерпретират,
вместо да се отпечатват като текст:

В showcase-а `F6` превключва фокуса между дървото и таблицата. `↑`/`↓`
местят реда само в активния панел; `Space` сменя отметка само в дървото.
`+` разгъва, а `-` сгъва избраните `Toolchains` или `Widgets`.
`←`/`→` отварят и сменят главното меню.

```powershell
.\demos\showcase\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-zig64 `
  -CompilerBin C:\zig `
  -Run
```

В него `↑/↓` сменят избрания ред, `Space` сменя отметка, `P` спира/пуска
анимацията, а `Q` или `Esc` затварят приложението.

## Готови бинарни файлове

Готовите Windows демота са в [`dist/`](dist/):

- `dist/showcase/showcase.exe` + `harbour_ratatui.dll` — интерактивното демо;
- `dist/zig64/demo_zig64.exe` + `harbour_ratatui.dll`;
- `dist/mingw64/demo_mingw64.exe` + `harbour_ratatui.dll`;
- `dist/mingw32/demo_mingw32.exe` + `harbour_ratatui.dll`.

Стартирайте `.exe` файла от собствената му папка, така че съответният DLL да е
до него. Бинарните файлове са само за бърз старт; build скриптовете остават
налични за повторна компилация от source.

## Как е свързано

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

За нов код се използва общият `HRC1` command-buffer interface. UI layout-ът,
данните, RGB стиловете и widget state-ът се описват в Harbour чрез
[`harbour/ratatui_builder.prg`](harbour/ratatui_builder.prg). Rust adapter-ът
само валидира и изпълнява командите. `RTUI_SHOWCASE*` функциите са оставени
само за обратна съвместимост и не са нужни за нови екрани.

DLL се зарежда динамично. Затова един x64 DLL работи както със Zig64, така и
с MinGW64, без значение от формата на техните import libraries. MinGW32
зарежда отделен i686 DLL. Паметта не се освобождава през чужд runtime:
Harbour подава caller-owned buffer, а Rust само го запълва.

## Изисквания

1. Windows 10 или по-нов за ANSI цветовете.
2. Актуален stable Rust (`rustup`, `cargo`, `rustc`).
3. MSVC Build Tools, нужни на Rust linker-а за `*-pc-windows-msvc` DLL.
4. Harbour пакет за съответната архитектура.
5. Zig или MinGW C compiler, съвпадащ с Harbour пакета.

MinGW32 и MinGW64 Harbour пакетите могат да се вземат от
[FiveTechSoft Harbour Builder](https://fivetechsoft.github.io/Harbour_builder/).
Разархивирайте ги в различни директории, например:

```text
C:\toolchains\harbour-mingw64
C:\toolchains\harbour-mingw32
```

Не смесвайте 32- и 64-битовите `bin`, `include` и `lib` директории.

Zig64 не е отделен download в тази страница. За него е нужен Harbour,
компилиран с compiler id `zig`, и 64-битов Zig в `PATH` или подаден с
`-CompilerBin`.

## Компилиране и стартиране

Всички команди се изпълняват от корена на repository-то в PowerShell.
`-CompilerBin` може да се пропусне, ако `zig.exe`/`gcc.exe` вече е в `PATH`.

### Zig64

```powershell
.\demos\zig64\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-zig64 `
  -CompilerBin C:\zig `
  -Run
```

Получавате:

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

Получавате:

```text
demos\mingw64\demo_mingw64.exe
demos\mingw64\harbour_ratatui.dll
```

### MinGW32

Еднократно добавете 32-битовата Rust standard library:

```powershell
rustup target add i686-pc-windows-msvc
```

После:

```powershell
.\demos\mingw32\build.ps1 `
  -HarbourRoot C:\toolchains\harbour-mingw32 `
  -CompilerBin C:\toolchains\mingw32\bin `
  -Run
```

Получавате:

```text
demos\mingw32\demo_mingw32.exe
demos\mingw32\harbour_ratatui.dll
```

При повторно Harbour компилиране може да пропуснете Cargo build-а:

```powershell
.\demos\zig64\build.ps1 -HarbourRoot C:\toolchains\harbour-zig64 -SkipRust
```

## Използване в собствен Harbour проект

Добавете C bridge файла в `.hbp`:

```text
myapp.prg
path/to/harbour_ratatui/harbour/hb_ratatui.c
```

Копирайте правилния `harbour_ratatui.dll` до `myapp.exe` и извикайте:

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

Публичният Harbour API е:

- `RTUI_AVAILABLE()` — зарежда DLL и проверява ABI;
- `RTUI_ABI_VERSION()` — връща текущата ABI версия (`1`);
- `RTUI_RENDER(cTitle, cBody, nWidth, nHeight, lAnsi)` — връща рендериран
  UTF-8 string или `NIL`;
- `RTUI_RENDER_COMMANDS(cBinary, lAnsi)` — изпълнява общия `HRC1` frame,
  създаден от Harbour builder-а;
- `RTUI_SHOWCASE(nTick, nSelected, nWidth, nHeight, lAnsi)` — рендерира
  базовия dashboard;
- `RTUI_SHOWCASE_EX(nTick, nSelected, nMenu, nMenuItem, nCheckedMask,`
  `lMenuOpen, nWidth, nHeight, lAnsi)` — рендерира интерактивния dashboard;
  Harbour управлява менюто, избрания ред и отметките;
- `RTUI_SHOWCASE_UI(nTick, nTreeSelected, nTableSelected, nFocus, nMenu,`
  `nMenuItem, nCheckedMask, lMenuOpen, nWidth, nHeight, lAnsi)` — добавя
  независим фокус и селекция за дървото и таблицата;
- `RTUI_SHOWCASE_TREE(nTick, nTreeSelected, nTableSelected, nFocus, nMenu,`
  `nMenuItem, nCheckedMask, nExpandedMask, lMenuOpen, nWidth, nHeight, lAnsi)`
  — добавя разгъване и сгъване на йерархичните групи;
- `RTUI_PRESENT(cUtf8, lAnsi)` — извежда UTF-8 frame-а чрез native
  `MultiByteToWideChar` + `WriteConsoleW`; при `.T.` включва Windows VT за
  24-bit RGB, а при `.F.` отказва низ, съдържащ ESC byte;
- `RTUI_LAST_ERROR()` — връща последната loader/Rust грешка;
- `RTUI_ENABLE_VT()` — включва ANSI обработка за Windows console output.

Ако DLL не е до executable файла, задайте абсолютния му път преди старт:

```powershell
$env:HB_RATATUI_DLL = 'D:\myapp\bin\harbour_ratatui.dll'
.\myapp.exe
```

`RTUI_RENDER()` приема ширина 24–500 и височина 9–200. Всички низове през
ABI са UTF-8 с изрична byte дължина; не са NUL-terminated параметри.

### Текстови модификатори

`RTUI_FRAME_PARAGRAPH()` приема незадължителен последен параметър
`nModifiers`. Маските `RTUI_MOD_BOLD`, `RTUI_MOD_DIM`, `RTUI_MOD_ITALIC`,
`RTUI_MOD_UNDERLINE`, `RTUI_MOD_BLINK`, `RTUI_MOD_REVERSE`,
`RTUI_MOD_CROSSED` и `RTUI_MOD_RAPID_BLINK` се комбинират със събиране:

```harbour
#include "ratatui.ch"

RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 1, 1, 40, 3 ), "", ;
   "Bold + italic + underline", aWhite, aBackground, aBorder, ;
   0, .F., .F., .F., ;
   RTUI_MOD_BOLD + RTUI_MOD_ITALIC + RTUI_MOD_UNDERLINE )
```

Старият `lBold` параметър остава валиден и се използва, когато
`nModifiers` не е подаден.

## Директни build команди

Скриптовете изпълняват еквивалента на:

```powershell
cargo build --release
cd demos\zig64
C:\toolchains\harbour-zig64\bin\hbmk2.exe demo_zig64.hbp -comp=zig -cpu=x86_64 -odemo_zig64
```

За MinGW64 сменете профила с `-comp=mingw64 -cpu=x86_64`, а за MinGW32 —
с `-comp=mingw -cpu=x86` и използвайте i686 DLL от
`target\i686-pc-windows-msvc\release`.

## Проверки

```powershell
cargo fmt --check
cargo test
cargo build --release
```

Rust export-ите са описани в `include/harbour_ratatui.h`. ABI е защитено от
version check, всички Rust panic-и се прихващат на FFI границата и към
Harbour не се подават Rust pointers или Rust-owned allocations.
