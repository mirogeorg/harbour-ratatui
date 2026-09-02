# Техническа осъществимост: Harbour bindings към Ratatui

Дата на проверката: 2026-09-01  
Обхват: архитектура и публичен API на Ratatui, backend модел, Rust FFI ограничения, лиценз, платформи и възможни стратегии за Harbour интеграция.  
Метод: официалните GitHub repository, Ratatui website, docs.rs, Rust Reference/Nomicon и Harbour repository бяха намерени и прочетени чрез Bright Data `search_engine` и `scrape_as_markdown`. Работната директория беше празна и нямаше установена конвенция за research notes, затова е използван `docs/research/`.

## Кратък отговор

**Да, bindings са технически осъществими, но не като директни 1:1 bindings към Rust API.** Практичният дизайн е Rust adapter crate, който зависи от Ratatui и експортира малък, версиониран C ABI (`staticlib` и/или `cdylib`). Harbour го извиква чрез тънък C слой, написан с Harbour Extend API.

Не е нужно непременно да започваме от нулата: community проектът [`holo-q/ratatui-ffi`](https://github.com/holo-q/ratatui-ffi) вече покрива голяма част от необходимия C ABI. Той е добър кандидат за **audit + fork + Harbour adapter**, но не трябва да се използва unmodified: текущият му `master` държи две несъвместими Crossterm версии, а допълнителните FFI safety checks са изключени по подразбиране.

Най-разумният първи продукт е **curated binding** за жизнения цикъл на терминала, layout, style, text и най-използваните widgets. Пълно огледало на целия Ratatui API би било скъпо за разработка и поддръжка, защото публичният Rust API е изграден около traits, generics, closures, lifetimes, builder-и и ownership, които нямат директен стабилен C ABI.

Оценка на осъществимостта:

| Вариант | Осъществимост | Сложност | Препоръка |
|---|---:|---:|---|
| Rust управлява терминала чрез Crossterm; Harbour подава view/commands | висока | средна | най-бърз MVP |
| Ratatui рендерира към Harbour GT чрез custom backend/callbacks | висока | средно-висока | най-добра native интеграция |
| Ratatui рендерира off-screen buffer; Harbour записва cell-овете | висока | средна | добър безопасен междинен вариант |
| Fork/адаптация на community `ratatui-ffi` | висока | средна | препоръчана начална база след audit |
| Автоматично 1:1 обвиване на целия Rust API | ниска | много висока | да не се прави |
| Отделен Rust процес с IPC | висока | висока оперативна сложност | fallback при проблеми с toolchain/ABI |

## Какво представлява Ratatui 0.30.2

Към датата на проверката публикуваната документация е за `ratatui 0.30.2`. От 0.30 проектът е разделен на workspace от специализирани crates:

- `ratatui-core`: `Widget`/`StatefulWidget`, text, buffer, layout, style и symbols;
- `ratatui-widgets`: стандартните widgets;
- отделни backend crates: `ratatui-crossterm`, `ratatui-termion`, `ratatui-termina`, `ratatui-termwiz`;
- `ratatui`: удобният facade crate, който re-export-ва останалите API-та.

Това разделяне е полезно за bindings: adapter-ът може да зависи само от `ratatui-core` + `ratatui-widgets`, ако Harbour ще управлява terminal I/O, или от facade crate + Crossterm, ако Rust ще управлява целия терминал. Официалният архитектурен документ описва dependency graph-а и посочва `ratatui-core` като стабилната основа за widget библиотеки: [ARCHITECTURE.md](https://github.com/ratatui/ratatui/blob/main/ARCHITECTURE.md).

Main crate по подразбиране включва всички widgets, Crossterm, layout cache, macros и underline color. Crossterm е default backend; Termion dependency е условно изключена на Windows. Същият manifest документира experimental feature flags, които не трябва да влизат в първата версия на binding-а: [ratatui/Cargo.toml](https://github.com/ratatui/ratatui/blob/main/ratatui/Cargo.toml).

Workspace manifest-ът към датата на проверката задава Rust edition 2024 и MSRV 1.88.0. Това е build-time изискване за нашия Rust adapter, не runtime изискване към потребителя: [Cargo.toml](https://github.com/ratatui/ratatui/blob/main/Cargo.toml).

## Rendering и публичен API

Ratatui е immediate-mode UI библиотека. Типичният flow е:

1. `Terminal::draw` приема Rust closure и създава `Frame`;
2. приложението извиква `Frame::render_widget`/`render_stateful_widget`;
3. widget-ите рисуват в междинен `Buffer` от `Cell` стойности;
4. `Terminal` сравнява текущия и предишния buffer и изпраща само разликите към backend-а.

`Cell` съдържа символ и отделна style информация (foreground/background/modifiers); един визуален cell не трябва да се приема за един byte или дори за един Unicode scalar. Официалното описание на pipeline-а и double buffering е в [Rendering under the hood](https://ratatui.rs/concepts/rendering/under-the-hood/), а version-pinned API-тата са в docs.rs за [`Terminal`](https://docs.rs/ratatui/0.30.2/ratatui/struct.Terminal.html), [`Buffer`](https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Buffer.html) и [`Cell`](https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Cell.html).

Публичните API области, които са подходящи за curated Harbour binding, са:

- geometry/layout: `Rect`, `Position`, `Size`, `Layout`, `Constraint`, `Direction`, `Flex`, `Margin`;
- style: `Color`, `Modifier`, `Style`, `Stylize` концепциите;
- text: `Span`, `Line`, `Text`, alignment и wrapping;
- widgets: `Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `BarChart`, `Chart`, `Canvas`, `Scrollbar`, `Sparkline`, `Calendar`;
- state: `ListState`, `TableState`, `ScrollbarState` и други stateful widget стойности;
- terminal: frame/draw, viewport, cursor, resize, clear;
- testing: `TestBackend` и детерминирани buffer snapshots.

Минималният `Widget` contract е по същество `render(self, area: Rect, buf: &mut Buffer)`, но параметрите и ownership моделът са Rust-specific: [Widget API](https://docs.rs/ratatui/0.30.2/ratatui/widgets/trait.Widget.html). Това е добра вътрешна seam точка за adapter-а, но не е подходящ ABI за директно излагане към Harbour.

## Backend abstraction

`Backend` абстрахира draw, cursor, clear, size/window size, flush и (условно) scrolling regions. `draw` приема generic iterator от `(u16, u16, &Cell)`, trait-ът има associated `Error` type и документацията изрично отбелязва, че **не е dyn-compatible**: [Backend trait 0.1.2](https://docs.rs/ratatui-core/0.1.2/ratatui_core/backend/trait.Backend.html).

Това има две последици:

1. Не можем просто да прехвърлим `Backend*` като универсален opaque trait object през C границата.
2. Можем без проблем да компилираме конкретен `Terminal<HarbourBackend>` вътре в Rust adapter-а. `HarbourBackend` ще е Rust struct с C-compatible callback table или собствен output buffer; generic-ът ще бъде monomorphized при компилация.

Официално поддържаните backend-и са Crossterm, Termion, Termwiz, Termina и `TestBackend`. Ratatui не поема целия application lifecycle: приложението обичайно използва съответната backend библиотека директно за keyboard/mouse/resize events, raw mode и alternate screen. Това трябва изрично да се покрие от wrapper API: [Ratatui Backends](https://ratatui.rs/concepts/backends/).

## Защо директни bindings не са подходящи

Самият официален Ratatui core не доставя или гарантира C ABI. Има community FFI crate, разгледан по-долу, но директното export-ване на официалните Rust типове остава нестабилно или невъзможно поради:

- generic типове и generic methods (`Terminal<B>`, `Backend::draw<I>`);
- traits и associated types;
- closures (`Terminal::draw`);
- lifetimes и borrowed references (`&Cell`, `&mut Buffer`, text със заети данни);
- `String`, `Vec`, builder типове и enums без изрично договорен C layout;
- ownership-consuming builder methods;
- `Backend` не е dyn-compatible;
- публичният API все още е pre-1.0 и проектът поддържа отделен списък с breaking changes: [BREAKING-CHANGES.md](https://github.com/ratatui/ratatui/blob/main/BREAKING-CHANGES.md).

Rust гарантира C-compatible layout само за внимателно дефинирани `#[repr(C)]` типове; нормалното Rust representation не е C договор. Официалните FFI насоки препоръчват C ABI functions, `#[repr(C)]`, raw/opaque pointers, ясна ownership дисциплина и предотвратяване на panic unwinding през FFI: [Rust Nomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html), [Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html), [Rust Reference: ABI](https://doc.rust-lang.org/reference/abi.html).

Следователно ABI-то трябва да съдържа само:

- fixed-width числа (`uint8_t`, `uint16_t`, `uint32_t`, `uint64_t`);
- opaque handles към Rust-owned objects;
- `#[repr(C)]` DTO structs с `struct_size`/`abi_version`;
- `(const uint8_t*, size_t)` за UTF-8 вход, копиран от Rust преди връщане;
- caller-provided buffers или симетрични `alloc/free` функции;
- integer status codes + `last_error`, без Rust `Result`, `String` или panic през границата.

## Съществуващ community проект: `ratatui-ffi`

Официално поддържаният от Ratatui общността каталог `awesome-ratatui` има раздел Bindings и включва [`ratatui-ffi`](https://github.com/holo-q/ratatui-ffi) като „FFI bindings for ratatui“: [awesome-ratatui Bindings](https://github.com/ratatui/awesome-ratatui#-bindings). Това е community listing, не обещание за support или ABI stability от Ratatui core maintainers.

Repository README-то заявява native C ABI `cdylib` и вече предлага:

- Paragraph, List/Table със state, Tabs, Gauge/LineGauge, BarChart, Sparkline, Chart, Scrollbar, Canvas и други widgets;
- layout split API, style/span/line DTO-та, RGB/indexed colors и modifiers;
- terminal init/clear, batched frame render, raw/alternate screen, cursor, size, event polling;
- headless snapshots и structured `FfiCellInfo` output;
- batching/reserve API-та, panic guards за terminal operations и `cbindgen` header generation.

Това е много близо до предложения в този документ MVP и значително намалява началната работа: [ratatui-ffi README](https://github.com/holo-q/ratatui-ffi/blob/master/README.md).

Има обаче три важни условия преди reuse:

1. **Dependency mismatch.** Публикуваният crate `0.2.6` зависи от `ratatui ^0.29` и `crossterm ^0.27`: [crates.io dependency metadata](https://crates.io/api/v1/crates/ratatui_ffi/0.2.6/dependencies). Текущият repository `master` е преминал към `ratatui = "0.30"`, но още декларира директно `crossterm = "0.27"`: [Cargo.toml](https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.toml). Понеже Ratatui 0.30.2 използва Crossterm 0.29 по подразбиране, lockfile-ът съдържа едновременно 0.27.0 и 0.29.0: [Cargo.lock](https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.lock).
2. **Това е функционален, не само козметичен риск.** Официалните Ratatui backend docs предупреждават, че несъвместими Crossterm версии имат отделни event queues и отделно raw-mode състояние, което може да доведе до race conditions, lost events и неправилно restore-ване: [Crossterm Version Compatibility](https://ratatui.rs/concepts/backends/#crossterm-version-compatibility).
3. **Safety policy.** README-то казва, че feature-ът `ffi_safety` е изключен по подразбиране и default build не компилира допълнителните checks. За Harbour integration той трябва да се включи в QA и най-вероятно в production, или всички entry points да минат отделен audit/fuzzing. Освен това част от string API-то приема NUL-terminated UTF-8; length-aware variants са по-безопасни за Harbour strings.

Препоръка за reuse:

- fork или pin към конкретен audited commit, не floating `master`;
- обновяване на директната Crossterm dependency до 0.29 и CI check, че `cargo tree -p crossterm` показва само една версия;
- `ratatui = "=0.30.2"` и отделно versioned C ABI;
- default-on bounds/pointer/length validation за Harbour artifact-а;
- запазване на batch/headless/widget implementation-а;
- добавяне на Harbour-specific C/Extend API слой и length-aware UTF-8 functions;
- отделяне или изключване на terminal/event ownership функциите, ако Harbour GT е собственик на терминала.

Публикуваният `ratatui_ffi 0.2.6` е MIT OR Apache-2.0 и crates.io го описва като C ABI binding, но е от 2025-09 и все още сочи Ratatui 0.29; repository state и release artifact не трябва да се смесват без изричен version/commit choice: [crates.io metadata](https://crates.io/api/v1/crates/ratatui_ffi).

## Препоръчана архитектура

### Слой 1: `ratatui-hb-ffi` (Rust)

Отделен crate — нов или audited fork на `ratatui-ffi` — който pin-ва точна Ratatui версия и се компилира като `staticlib`, `cdylib` или и двете. Rust Reference определя `staticlib` като формат, препоръчан за вграждане на Rust в съществуващо non-Rust приложение, а `cdylib` като dynamic system library за зареждане от друг език: [Rust Reference: linkage](https://doc.rust-lang.org/reference/linkage.html).

Adapter-ът:

- притежава всички Ratatui/Rust обекти;
- експортира само C ABI functions;
- преобразува C DTO-та към Ratatui builders за текущия frame;
- прихваща unwind panic с `catch_unwind` и го превръща в status/error text;
- гарантира terminal restore при normal close и при обработими грешки;
- не допуска exception/panic да преминава ABI границата;
- пази ABI версия отделно от Ratatui версията.

`cbindgen` може да генерира C header за нашите `extern "C"` DTO-та и functions, но не може да превърне самия generic Ratatui API в C ABI. Header generation е помощно средство, не binding стратегия.

### Слой 2: `hb_ratatui.c` (Harbour Extend API)

Тънък C wrapper дефинира `HB_FUNC(...)` entry points, валидира Harbour аргументите, извиква C ABI и преобразува status/error резултатите към Harbour. Официалният Harbour `hbapi.h` е header-ът за Extend API, Array API и основните VM декларации: [Harbour hbapi.h](https://github.com/harbour/core/blob/master/include/hbapi.h).

`hbmk2` поддържа допълнителни library paths, библиотеки и static/shared linking, което е достатъчно за свързване на Rust artifact-а: [hbmk2 documentation](https://github.com/harbour/core/blob/master/utils/hbmk2/doc/hbmk2.en.md).

### Слой 3: Harbour-friendly API

Да не се копира Rust builder API 1:1. По-удобен Harbour interface би използвал hashes/arrays или малки Harbour classes, например концептуално:

```text
h := RTUI_New( { "backend" => "crossterm" } )
RTUI_BeginFrame( h )
aRects := RTUI_Layout( h, area, constraints )
RTUI_Paragraph( h, aRects[ 1 ], text, style, options )
RTUI_List( h, aRects[ 2 ], items, state, options )
RTUI_EndFrame( h )
event := RTUI_PollEvent( h, timeout_ms )
RTUI_Close( h )
```

Вътрешно `BeginFrame`/`EndFrame` може да събира command list и да изпълни един Rust `Terminal::draw` closure. Не трябва да държим `&mut Frame` през отделни Harbour calls; Rust borrow не може безопасно да пресече и да остане жив през C/VM границата.

## Стратегии за terminal I/O

### A. Rust-owned Crossterm backend — препоръчано за MVP

Rust adapter-ът притежава `Terminal<CrosstermBackend<...>>`, raw mode, alternate screen и normalized input events. Harbour подава widget commands/state.

Плюсове:

- използва стандартния, default Ratatui path;
- получава diff rendering и terminal lifecycle на едно място;
- най-малко custom backend код;
- работи на Linux, macOS и Windows според Ratatui manifest-а и Crossterm support-а.

Минуси:

- трябва ясно да се договори кой притежава конзолата;
- може да конфликтува със съществуващ Harbour GT driver, ако и двата пишат/четат едновременно;
- input events трябва да се нормализират в собствен C enum/DTO, а не да се export-ват Crossterm enums.

Crossterm декларира поддръжка на UNIX и Windows terminals, включително Windows 7 с някои feature ограничения; README-то описва cursor, style, raw/alternate screen и key/mouse/resize events: [Crossterm README](https://github.com/crossterm-rs/crossterm/blob/master/README.md).

### B. `Terminal<HarbourBackend>` — препоръчано за дълбока GT интеграция

В Rust имплементираме конкретен `HarbourBackend` с C callback table за size, draw cells, cursor, clear и flush. Adapter-ът flatten-ва iterator-а от Ratatui `Cell` в краткотраен C array и извиква callbacks.

Плюсове:

- Harbour/GT остава собственик на terminal I/O и input;
- запазва Ratatui double-buffer diffing;
- може да се интегрира с конкретен Harbour TrueColor/GT слой.

Минуси и условия:

- повече backend код и platform testing;
- callback-ите трябва да са synchronous и по възможност на същия thread;
- callback pointer-ите и context handle трябва да останат валидни до `close`;
- да няма Harbour VM re-entry от чужд thread;
- wide Unicode grapheme, continuation cells, RGB/indexed colors и modifiers трябва да имат точен DTO contract.

`Backend` методите и non-dyn ограничението не пречат, защото конкретният `HarbourBackend` се monomorphize-ва вътре в Rust binary/library.

### C. Off-screen `Buffer`/`TestBackend`

Rust рендерира widgets към memory buffer и връща full frame или diff от C-compatible cells; Harbour ги записва чрез GT API. `TestBackend` е официално предоставен за UI testing, но production adapter може да използва собствен memory backend или директно `Buffer`, за да не зависи от test-only semantics.

Плюсове:

- няма callbacks към Harbour;
- отлична testability и детерминирани snapshots;
- ясно отделя layout/widget engine от terminal I/O.

Минуси:

- Harbour слой трябва да прилага cells и cursor сам;
- ако се връща full frame, трафикът е по-голям; по-добре е adapter-ът да поддържа предишен buffer и да връща diff;
- трябва да се синхронизира resize.

### D. IPC helper process

Rust executable управлява Ratatui, а Harbour комуникира чрез pipes/socket с versioned protocol. Това елиминира in-process ABI и panic рисковете, но добавя process lifecycle, IPC latency и packaging. Подходящо е само ако C/Rust toolchain съвместимостта се окаже блокираща.

## Platform и build ограничения

- Default Crossterm конфигурацията е целена към Linux/macOS/Windows; Termion е Unix-only в manifest-а. За първа версия използваме Crossterm или Harbour-owned backend, не множество backend-и.
- Архитектурата трябва да съвпада: x86 с x86, x64 с x64, ARM64 с ARM64.
- На Windows Rust target/toolchain трябва да съвпада с C linker ecosystem-а на Harbour build-а (`*-pc-windows-msvc` срещу `*-pc-windows-gnu`). При несъвпадение `cdylib` обикновено изолира повече link-time детайли, но остава необходима подходяща import library или runtime loading.
- `staticlib` вгражда Rust dependencies, но системните native libraries трябва да се подадат на крайния linker; Rust препоръчва `--print=native-static-libs` за този списък: [Rust linkage](https://doc.rust-lang.org/reference/linkage.html).
- `cdylib` опростява Harbour executable link-а и независимото обновяване, но добавя DLL/SO/DYLIB deployment и ABI version management.
- За binary distribution трябва CI matrix по поддържана OS/architecture/toolchain, а не компилация на машината на крайния потребител.

Практична препоръка: MVP с `cdylib` за по-малко linker coupling, след това и `staticlib` за toolchain-и, при които крайната връзка е доказано стабилна.

### Проверка на локалния Windows toolchain

Локалната проверка на 2026-09-01 показа:

- Harbour `hbmk2` автоматично избира 64-bit Zig toolchain от `D:\accounts\hb32_64_v3`;
- Zig 0.16.0 докладва GNU Windows target (`x86_64-windows-gnu`);
- инсталираният Rust host/target е само `x86_64-pc-windows-msvc`.

Не е необходимо Harbour да се прекомпилира с друг C compiler. Предпочитаният build е да се добави Rust target `x86_64-pc-windows-gnu` и Rust `cdylib`-ът да се произведе за него, така че Harbour C shim-ът и DLL import library да са в една ABI/linker фамилия. Ако GNU Rust target-ът създаде конкретен linker проблем, резервният вариант е MSVC Rust DLL с runtime loading (`LoadLibrary`/`GetProcAddress`), което премахва зависимостта от import-library формата. Harbour/MSVC build е последен fallback, не начално изискване.

Независимо от toolchain избора, allocator ownership не трябва да пресича DLL границата: Rust-allocated обекти/strings се освобождават от Rust export, а Harbour-allocated памет — от Harbour.

## FFI safety contract

Задължителни правила:

1. **Opaque ownership:** Harbour държи handle; само Rust създава/унищожава Rust обектите.
2. **No unwind:** всяка exported function има panic guard; връща status code.
3. **Strings:** входът е UTF-8 pointer + byte length; Rust го копира, ако трябва да живее след call-а. Няма NUL assumption.
4. **Memory symmetry:** памет, заделена от Rust, се освобождава само с Rust `rtui_free_*`; още по-добре — caller-provided buffers.
5. **Thread affinity:** session handle се използва само от thread-а, който го е създал, освен ако бъде изрично проектиран и тестван другояче.
6. **Callbacks:** synchronous, без задържане на временни pointers след callback-а, без exception през C границата.
7. **Terminal restore:** idempotent `close`; cleanup при частично неуспешен `open`; ясно поведение при Harbour error/quit.
8. **Versioning:** `rtui_abi_version()`, `struct_size` и feature query. Ratatui остава скрит implementation detail.
9. **Limits:** валидиране на координати/размери (`u16` в Ratatui), array lengths, UTF-8 и enum стойности преди Rust API call.
10. **No borrowed frame:** никой `Frame`, `Buffer`, `Cell`, `&str` или Rust iterator не се връща директно към Harbour.

## Предложен MVP

Първата binding версия да покрива:

- session: create/open, close/restore, terminal size, clear;
- frame: begin/end или една `render(command_list)` функция;
- layout: horizontal/vertical constraints и `Rect`;
- style: default, named/ANSI/indexed/RGB colors, modifiers;
- text: UTF-8 text, spans/lines, alignment, wrap;
- widgets: `Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `Scrollbar`;
- state: selected row/item и scroll position като прости integers/DTO-та;
- input: key, mouse, resize, focus/paste само ако Rust-owned Crossterm е избран;
- errors: status + copied last-error text;
- testing: off-screen rendering и golden snapshots.

Да се отложат за следваща фаза:

- `Canvas`, arbitrary closures и custom widget callbacks;
- experimental `WidgetRef` APIs;
- множество terminal backends в един build;
- generic third-party widget loading;
- директно export-ване на serialized Ratatui internal types.

Примерна ниско-нивова C ABI форма:

```c
typedef struct rtui_session rtui_session;

typedef struct {
    uint32_t struct_size;
    uint32_t abi_version;
    uint16_t x, y, width, height;
} rtui_rect;

int32_t rtui_session_new(const rtui_options *opt, rtui_session **out);
int32_t rtui_frame_begin(rtui_session *s);
int32_t rtui_paragraph(rtui_session *s, rtui_rect area,
                       const uint8_t *utf8, size_t len,
                       const rtui_paragraph_options *opt);
int32_t rtui_frame_end(rtui_session *s);
int32_t rtui_poll_event(rtui_session *s, uint32_t timeout_ms,
                        rtui_event *out);
void    rtui_session_free(rtui_session *s);
```

Точните signatures трябва да се фиксират след малък prototype; горното показва ABI формата, не окончателен interface.

## Рискове и mitigations

| Риск | Ефект | Mitigation |
|---|---|---|
| Ratatui breaking changes | чести промени във wrapper implementation | pin точна версия; нашият C ABI е независим; upgrade tests |
| Две Crossterm версии в community fork | разделени event queues/raw-mode state | pin Crossterm 0.29; `cargo tree` CI assertion; един terminal owner |
| Rust/Harbour linker несъвместимост | build failure, особено Windows | CI по toolchain; `cdylib` MVP; отделни GNU/MSVC artifacts |
| Двама собственици на terminal state | повреден raw mode/alternate screen/input | един session owner; изричен open/close contract |
| Panic през FFI | abort/undefined behavior според ABI сценария | `catch_unwind`, status codes, panic-free boundary |
| Unicode/codepage разлика | счупени glyphs/width | UTF-8 ABI; conversion само в Harbour edge; tests с wide/combining graphemes |
| Callback lifetime/threading | use-after-free или VM corruption | opaque context, same-thread synchronous callbacks, unregister-before-free |
| 1:1 API explosion | неподлежаща на поддръжка binding surface | curated Harbour API и command DTO-та |
| Transitive dependency licenses | packaging compliance | license inventory при release; пазене на notices |

## Лиценз

Ratatui е под MIT лиценз. Той разрешава use, copy, modify, distribute, sublicense и sale, при условие че copyright и permission notice се запазят в copies/substantial portions: [официален LICENSE](https://github.com/ratatui/ratatui/blob/main/LICENSE).

Това е съвместимо с proprietary или open-source Harbour приложение. Все пак binary release трябва да включва Ratatui MIT notice и да има автоматизиран license audit за всички transitive Cargo dependencies; Ratatui лицензът сам по себе си не удостоверява лицензите на цялото dependency дърво.

## Решение и следваща проверка

**Go за prototype.** Няма архитектурен или лицензионен blocker, а `ratatui-ffi` вече доказва C ABI подхода и предлага голяма част от желаната surface area. Основният избор е дали да fork-нем него или да използваме по-малък adapter, както и ownership на terminal I/O:

- ако целта е бързо standalone Harbour TUI: Rust-owned Crossterm;
- ако целта е интеграция със съществуващ Harbour GT/TrueColor stack: `HarbourBackend` callbacks или off-screen buffer.

Препоръчан технически spike:

1. кратък security/API audit на `ratatui-ffi`, следван от fork или thin wrapper, pin-нат към конкретен commit, `ratatui = "=0.30.2"` и единствен `crossterm = "0.29"`;
2. `cdylib` + C header с 8–12 exported functions;
3. Harbour demo с `Block` + `Paragraph`, RGB style, Unicode, resize и clean restore;
4. off-screen test, който проверява точните cells;
5. Windows x64 и Linux x64 build; проверка на Harbour compiler/linker варианта преди static linking;
6. след spike-а — избор между command-list API и конкретни widget functions.

Acceptance criteria за spike-а:

- terminal-ът се възстановява след нормално излизане и обработима грешка;
- 1 000 frame цикъла без leak/use-after-free според sanitizers/Valgrind еквивалент;
- UTF-8, RGB/indexed colors и wide glyph test минават;
- resize не поврежда buffer/state;
- C ABI header остава без Rust-specific типове;
- същият Harbour-facing API работи с поне един Windows и един Unix artifact.

## Първични източници

- Ratatui repository и README: <https://github.com/ratatui/ratatui>
- Ratatui architecture: <https://github.com/ratatui/ratatui/blob/main/ARCHITECTURE.md>
- Ratatui 0.30.2 `Terminal`: <https://docs.rs/ratatui/0.30.2/ratatui/struct.Terminal.html>
- Ratatui 0.30.2 `Buffer`/`Cell`: <https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Buffer.html>, <https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Cell.html>
- `ratatui-core` `Backend`: <https://docs.rs/ratatui-core/0.1.2/ratatui_core/backend/trait.Backend.html>
- Rendering pipeline: <https://ratatui.rs/concepts/rendering/under-the-hood/>
- Backends: <https://ratatui.rs/concepts/backends/>
- Ratatui manifests: <https://github.com/ratatui/ratatui/blob/main/Cargo.toml>, <https://github.com/ratatui/ratatui/blob/main/ratatui/Cargo.toml>
- Ratatui MIT license: <https://github.com/ratatui/ratatui/blob/main/LICENSE>
- Rust FFI and ABI: <https://doc.rust-lang.org/nomicon/ffi.html>, <https://doc.rust-lang.org/reference/type-layout.html>, <https://doc.rust-lang.org/reference/abi.html>, <https://doc.rust-lang.org/reference/linkage.html>
- Harbour Extend API: <https://github.com/harbour/core/blob/master/include/hbapi.h>
- Harbour build/link tool: <https://github.com/harbour/core/blob/master/utils/hbmk2/doc/hbmk2.en.md>
- Crossterm platform/features: <https://github.com/crossterm-rs/crossterm/blob/master/README.md>
- Ratatui community bindings catalog: <https://github.com/ratatui/awesome-ratatui#-bindings>
- Community `ratatui-ffi`: <https://github.com/holo-q/ratatui-ffi>, <https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.toml>, <https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.lock>
- Published `ratatui_ffi 0.2.6`: <https://crates.io/api/v1/crates/ratatui_ffi>, <https://crates.io/api/v1/crates/ratatui_ffi/0.2.6/dependencies>
