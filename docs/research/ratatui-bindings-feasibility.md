# Technical feasibility: Harbour bindings for Ratatui

Review date: 2026-09-01<br>
Scope: Ratatui architecture and public API, backend model, Rust FFI constraints, licensing, platforms, and possible Harbour integration strategies.<br>
Method: the official GitHub repositories, Ratatui website, docs.rs, Rust Reference/Nomicon, and Harbour repository were found and reviewed using Bright Data `search_engine` and `scrape_as_markdown`. The working directory was empty and had no established convention for research notes, so `docs/research/` was used.

## Short answer

**Yes, bindings are technically feasible, but not as direct 1:1 bindings to the Rust API.** The practical design is a Rust adapter crate that depends on Ratatui and exports a small, versioned C ABI (`staticlib` and/or `cdylib`). Harbour calls it through a thin C layer written with the Harbour Extend API.

There is no need to start entirely from scratch: the community project [`holo-q/ratatui-ffi`](https://github.com/holo-q/ratatui-ffi) already covers much of the required C ABI. It is a good candidate for an **audit + fork + Harbour adapter**, but should not be used unmodified: its current `master` contains two incompatible Crossterm versions, and the additional FFI safety checks are disabled by default.

The most sensible first product is a **curated binding** for the terminal lifecycle, layout, style, text, and the most commonly used widgets. A complete mirror of the entire Ratatui API would be expensive to develop and maintain because the public Rust API is built around traits, generics, closures, lifetimes, builders, and ownership, which have no direct stable C ABI.

Feasibility assessment:

| Option | Feasibility | Complexity | Recommendation |
|---|---:|---:|---|
| Rust manages the terminal through Crossterm; Harbour supplies views/commands | high | medium | fastest MVP |
| Ratatui renders to Harbour GT through a custom backend/callbacks | high | medium-high | best native integration |
| Ratatui renders to an off-screen buffer; Harbour writes the cells | high | medium | good, safe intermediate option |
| Fork/adapt the community `ratatui-ffi` | high | medium | recommended starting point after an audit |
| Automatically wrap the entire Rust API 1:1 | low | very high | do not pursue |
| Separate Rust process with IPC | high | high operational complexity | fallback for toolchain/ABI problems |

## What Ratatui 0.30.2 is

At the time of the review, the published documentation covered `ratatui 0.30.2`. Since 0.30, the project has been organized as a workspace of specialized crates:

- `ratatui-core`: `Widget`/`StatefulWidget`, text, buffer, layout, style, and symbols;
- `ratatui-widgets`: the standard widgets;
- separate backend crates: `ratatui-crossterm`, `ratatui-termion`, `ratatui-termina`, and `ratatui-termwiz`;
- `ratatui`: the convenient facade crate that re-exports the other APIs.

This separation is useful for bindings: the adapter can depend only on `ratatui-core` + `ratatui-widgets` if Harbour manages terminal I/O, or on the facade crate + Crossterm if Rust manages the entire terminal. The official architecture document describes the dependency graph and identifies `ratatui-core` as the stable foundation for widget libraries: [ARCHITECTURE.md](https://github.com/ratatui/ratatui/blob/main/ARCHITECTURE.md).

By default, the main crate includes all widgets, Crossterm, the layout cache, macros, and underline color. Crossterm is the default backend; the Termion dependency is conditionally excluded on Windows. The same manifest documents experimental feature flags that should not be included in the first binding version: [ratatui/Cargo.toml](https://github.com/ratatui/ratatui/blob/main/ratatui/Cargo.toml).

At the time of the review, the workspace manifest specified Rust edition 2024 and MSRV 1.88.0. This is a build-time requirement for our Rust adapter, not a runtime requirement for users: [Cargo.toml](https://github.com/ratatui/ratatui/blob/main/Cargo.toml).

## Rendering and public API

Ratatui is an immediate-mode UI library. The typical flow is:

1. `Terminal::draw` accepts a Rust closure and creates a `Frame`;
2. the application calls `Frame::render_widget`/`render_stateful_widget`;
3. widgets draw into an intermediate `Buffer` of `Cell` values;
4. `Terminal` compares the current and previous buffers and sends only the differences to the backend.

`Cell` contains a symbol and separate style information (foreground/background/modifiers); one visual cell must not be assumed to equal one byte or even one Unicode scalar. The official description of the pipeline and double buffering is in [Rendering under the hood](https://ratatui.rs/concepts/rendering/under-the-hood/), while version-pinned APIs are available on docs.rs for [`Terminal`](https://docs.rs/ratatui/0.30.2/ratatui/struct.Terminal.html), [`Buffer`](https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Buffer.html), and [`Cell`](https://docs.rs/ratatui/0.30.2/ratatui/buffer/struct.Cell.html).

The public API areas suitable for a curated Harbour binding are:

- geometry/layout: `Rect`, `Position`, `Size`, `Layout`, `Constraint`, `Direction`, `Flex`, `Margin`;
- style: the `Color`, `Modifier`, `Style`, and `Stylize` concepts;
- text: `Span`, `Line`, `Text`, alignment, and wrapping;
- widgets: `Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `BarChart`, `Chart`, `Canvas`, `Scrollbar`, `Sparkline`, `Calendar`;
- state: `ListState`, `TableState`, `ScrollbarState`, and other stateful widget values;
- terminal: frame/draw, viewport, cursor, resize, clear;
- testing: `TestBackend` and deterministic buffer snapshots.

The minimal `Widget` contract is essentially `render(self, area: Rect, buf: &mut Buffer)`, but its parameters and ownership model are Rust-specific: [Widget API](https://docs.rs/ratatui/0.30.2/ratatui/widgets/trait.Widget.html). This is a good internal seam for the adapter, but not a suitable ABI to expose directly to Harbour.

## Backend abstraction

`Backend` abstracts drawing, cursor operations, clearing, size/window size, flushing, and, conditionally, scrolling regions. `draw` accepts a generic iterator over `(u16, u16, &Cell)`; the trait has an associated `Error` type, and the documentation explicitly notes that it is **not dyn-compatible**: [Backend trait 0.1.2](https://docs.rs/ratatui-core/0.1.2/ratatui_core/backend/trait.Backend.html).

This has two consequences:

1. We cannot simply pass a `Backend*` as a universal opaque trait object across the C boundary.
2. We can compile a concrete `Terminal<HarbourBackend>` inside the Rust adapter without difficulty. `HarbourBackend` would be a Rust struct with a C-compatible callback table or its own output buffer; the generic would be monomorphized at compile time.

The officially supported backends are Crossterm, Termion, Termwiz, Termina, and `TestBackend`. Ratatui does not own the entire application lifecycle: applications normally use the corresponding backend library directly for keyboard/mouse/resize events, raw mode, and the alternate screen. The wrapper API must cover these responsibilities explicitly: [Ratatui Backends](https://ratatui.rs/concepts/backends/).

## Why direct bindings are unsuitable

The official Ratatui core itself does not provide or guarantee a C ABI. A community FFI crate is discussed below, but directly exporting the official Rust types remains unstable or impossible because of:

- generic types and generic methods (`Terminal<B>`, `Backend::draw<I>`);
- traits and associated types;
- closures (`Terminal::draw`);
- lifetimes and borrowed references (`&Cell`, `&mut Buffer`, text containing borrowed data);
- `String`, `Vec`, builder types, and enums without an explicitly agreed C layout;
- ownership-consuming builder methods;
- `Backend` not being dyn-compatible;
- the public API still being pre-1.0, with the project maintaining a separate list of breaking changes: [BREAKING-CHANGES.md](https://github.com/ratatui/ratatui/blob/main/BREAKING-CHANGES.md).

Rust guarantees C-compatible layout only for carefully defined `#[repr(C)]` types; normal Rust representation is not a C contract. The official FFI guidance recommends C ABI functions, `#[repr(C)]`, raw/opaque pointers, explicit ownership discipline, and preventing panic unwinding across FFI: [Rust Nomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html), [Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html), [Rust Reference: ABI](https://doc.rust-lang.org/reference/abi.html).

Therefore, the ABI should contain only:

- fixed-width integers (`uint8_t`, `uint16_t`, `uint32_t`, `uint64_t`);
- opaque handles to Rust-owned objects;
- `#[repr(C)]` DTO structs with `struct_size`/`abi_version`;
- `(const uint8_t*, size_t)` for UTF-8 input, copied by Rust before returning;
- caller-provided buffers or symmetric `alloc/free` functions;
- integer status codes + `last_error`, with no Rust `Result`, `String`, or panic crossing the boundary.

## Existing community project: `ratatui-ffi`

The officially maintained Ratatui community catalog, `awesome-ratatui`, has a Bindings section and includes [`ratatui-ffi`](https://github.com/holo-q/ratatui-ffi) as “FFI bindings for ratatui”: [awesome-ratatui Bindings](https://github.com/ratatui/awesome-ratatui#-bindings). This is a community listing, not a promise of support or ABI stability from the Ratatui core maintainers.

The repository README describes a native C ABI `cdylib` and already offers:

- Paragraph, stateful List/Table, Tabs, Gauge/LineGauge, BarChart, Sparkline, Chart, Scrollbar, Canvas, and other widgets;
- a layout split API, style/span/line DTOs, RGB/indexed colors, and modifiers;
- terminal initialization/clear, batched frame rendering, raw/alternate screen, cursor, size, and event polling;
- headless snapshots and structured `FfiCellInfo` output;
- batching/reserve APIs, panic guards for terminal operations, and `cbindgen` header generation.

This is very close to the MVP proposed in this document and significantly reduces the initial work: [ratatui-ffi README](https://github.com/holo-q/ratatui-ffi/blob/master/README.md).

There are, however, three important conditions before reuse:

1. **Dependency mismatch.** The published crate `0.2.6` depends on `ratatui ^0.29` and `crossterm ^0.27`: [crates.io dependency metadata](https://crates.io/api/v1/crates/ratatui_ffi/0.2.6/dependencies). The current repository `master` has moved to `ratatui = "0.30"`, but still declares a direct `crossterm = "0.27"` dependency: [Cargo.toml](https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.toml). Because Ratatui 0.30.2 uses Crossterm 0.29 by default, the lockfile contains both 0.27.0 and 0.29.0: [Cargo.lock](https://github.com/holo-q/ratatui-ffi/blob/master/Cargo.lock).
2. **This is a functional, not merely cosmetic, risk.** The official Ratatui backend documentation warns that incompatible Crossterm versions have separate event queues and separate raw-mode state, which may lead to race conditions, lost events, and incorrect restoration: [Crossterm Version Compatibility](https://ratatui.rs/concepts/backends/#crossterm-version-compatibility).
3. **Safety policy.** The README says that the `ffi_safety` feature is disabled by default and that the default build does not compile the additional checks. For Harbour integration, it should be enabled in QA and most likely in production, or all entry points should undergo a separate audit/fuzzing effort. In addition, some of the string API accepts NUL-terminated UTF-8; length-aware variants are safer for Harbour strings.

Reuse recommendation:

- fork or pin a specific audited commit, not floating `master`;
- update the direct Crossterm dependency to 0.29 and add a CI check that `cargo tree -p crossterm` shows only one version;
- use `ratatui = "=0.30.2"` and a separately versioned C ABI;
- enable bounds/pointer/length validation by default for the Harbour artifact;
- preserve the batch/headless/widget implementation;
- add a Harbour-specific C/Extend API layer and length-aware UTF-8 functions;
- separate or disable terminal/event ownership functions if Harbour GT owns the terminal.

The published `ratatui_ffi 0.2.6` is licensed MIT OR Apache-2.0 and is described on crates.io as a C ABI binding, but it dates from 2025-09 and still targets Ratatui 0.29. The repository state and release artifact must not be mixed without an explicit version/commit choice: [crates.io metadata](https://crates.io/api/v1/crates/ratatui_ffi).

## Recommended architecture

### Layer 1: `ratatui-hb-ffi` (Rust)

A separate crate—either new or an audited fork of `ratatui-ffi`—pins an exact Ratatui version and compiles as a `staticlib`, `cdylib`, or both. The Rust Reference defines `staticlib` as the format recommended for embedding Rust in an existing non-Rust application, and `cdylib` as a dynamic system library for loading from another language: [Rust Reference: linkage](https://doc.rust-lang.org/reference/linkage.html).

The adapter:

- owns all Ratatui/Rust objects;
- exports only C ABI functions;
- converts C DTOs to Ratatui builders for the current frame;
- catches unwinding panics with `catch_unwind` and converts them to status/error text;
- guarantees terminal restoration on normal close and recoverable errors;
- prevents exceptions/panics from crossing the ABI boundary;
- versions the ABI independently from the Ratatui version.

`cbindgen` can generate a C header for our `extern "C"` DTOs and functions, but it cannot turn the generic Ratatui API itself into a C ABI. Header generation is a supporting tool, not a binding strategy.

### Layer 2: `hb_ratatui.c` (Harbour Extend API)

A thin C wrapper defines `HB_FUNC(...)` entry points, validates Harbour arguments, calls the C ABI, and converts status/error results for Harbour. The official Harbour `hbapi.h` is the header for the Extend API, Array API, and core VM declarations: [Harbour hbapi.h](https://github.com/harbour/core/blob/master/include/hbapi.h).

`hbmk2` supports additional library paths, libraries, and static/shared linking, which is sufficient for linking the Rust artifact: [hbmk2 documentation](https://github.com/harbour/core/blob/master/utils/hbmk2/doc/hbmk2.en.md).

### Layer 3: Harbour-friendly API

Do not copy the Rust builder API 1:1. A more convenient Harbour interface would use hashes/arrays or small Harbour classes, conceptually:

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

Internally, `BeginFrame`/`EndFrame` can collect a command list and execute a single Rust `Terminal::draw` closure. We must not hold `&mut Frame` across separate Harbour calls; a Rust borrow cannot safely cross and remain alive across the C/VM boundary.

## Terminal I/O strategies

### A. Rust-owned Crossterm backend—recommended for the MVP

The Rust adapter owns `Terminal<CrosstermBackend<...>>`, raw mode, the alternate screen, and normalized input events. Harbour supplies widget commands/state.

Advantages:

- uses the standard, default Ratatui path;
- provides diff rendering and terminal lifecycle management in one place;
- requires the least custom backend code;
- works on Linux, macOS, and Windows according to the Ratatui manifest and Crossterm support.

Disadvantages:

- console ownership must be defined clearly;
- it may conflict with an existing Harbour GT driver if both read/write concurrently;
- input events must be normalized into a custom C enum/DTO instead of exporting Crossterm enums.

Crossterm declares support for UNIX and Windows terminals, including Windows 7 with some feature limitations. Its README describes cursor, style, raw/alternate screen, and key/mouse/resize events: [Crossterm README](https://github.com/crossterm-rs/crossterm/blob/master/README.md).

### B. `Terminal<HarbourBackend>`—recommended for deep GT integration

In Rust, we implement a concrete `HarbourBackend` with a C callback table for size, drawing cells, cursor operations, clearing, and flushing. The adapter flattens the Ratatui `Cell` iterator into a short-lived C array and calls the callbacks.

Advantages:

- Harbour/GT remains the owner of terminal I/O and input;
- preserves Ratatui double-buffer diffing;
- can integrate with a specific Harbour TrueColor/GT layer.

Disadvantages and conditions:

- more backend code and platform testing;
- callbacks must be synchronous and preferably run on the same thread;
- callback pointers and the context handle must remain valid until `close`;
- there must be no Harbour VM re-entry from a foreign thread;
- wide Unicode graphemes, continuation cells, RGB/indexed colors, and modifiers require a precise DTO contract.

The `Backend` methods and the non-dyn restriction are not obstacles because the concrete `HarbourBackend` is monomorphized inside the Rust binary/library.

### C. Off-screen `Buffer`/`TestBackend`

Rust renders widgets into a memory buffer and returns a full frame or diff of C-compatible cells; Harbour writes them through the GT API. `TestBackend` is officially provided for UI testing, but a production adapter can use its own memory backend or `Buffer` directly to avoid depending on test-only semantics.

Advantages:

- no callbacks into Harbour;
- excellent testability and deterministic snapshots;
- cleanly separates the layout/widget engine from terminal I/O.

Disadvantages:

- the Harbour layer must apply cells and cursor operations itself;
- returning a full frame creates more traffic; it is better for the adapter to retain the previous buffer and return a diff;
- resize must be synchronized.

### D. IPC helper process

A Rust executable manages Ratatui while Harbour communicates through pipes/a socket using a versioned protocol. This eliminates in-process ABI and panic risks, but adds process lifecycle, IPC latency, and packaging complexity. It is appropriate only if C/Rust toolchain compatibility proves to be a blocker.

## Platform and build constraints

- The default Crossterm configuration targets Linux/macOS/Windows; Termion is Unix-only in the manifest. For the first version, use Crossterm or a Harbour-owned backend, not multiple backends.
- Architectures must match: x86 with x86, x64 with x64, ARM64 with ARM64.
- On Windows, the Rust target/toolchain must match the C linker ecosystem of the Harbour build (`*-pc-windows-msvc` versus `*-pc-windows-gnu`). When they do not match, a `cdylib` usually isolates more link-time details, but a suitable import library or runtime loading is still required.
- `staticlib` embeds Rust dependencies, but system-native libraries must be passed to the final linker; Rust recommends `--print=native-static-libs` for this list: [Rust linkage](https://doc.rust-lang.org/reference/linkage.html).
- `cdylib` simplifies linking the Harbour executable and independent updates, but adds DLL/SO/DYLIB deployment and ABI version management.
- Binary distribution requires a CI matrix covering each supported OS/architecture/toolchain, rather than compilation on the end user's machine.

Practical recommendation: use a `cdylib` for the MVP to reduce linker coupling, then add a `staticlib` for toolchains where final linking has proven stable.

### Local Windows toolchain review

The local review on 2026-09-01 found:

- Harbour `hbmk2` automatically selects the 64-bit Zig toolchain from `D:\accounts\hb32_64_v3`;
- Zig 0.16.0 reports a GNU Windows target (`x86_64-windows-gnu`);
- the only installed Rust host/target is `x86_64-pc-windows-msvc`.

Harbour does not need to be rebuilt with a different C compiler. The preferred build adds the Rust target `x86_64-pc-windows-gnu` and builds the Rust `cdylib` for it, keeping the Harbour C shim and DLL import library in the same ABI/linker family. If the GNU Rust target causes a specific linker problem, the fallback is an MSVC Rust DLL loaded at runtime with `LoadLibrary`/`GetProcAddress`, removing the dependency on the import-library format. A Harbour/MSVC build is the last fallback, not the initial requirement.

Regardless of the toolchain choice, allocator ownership must not cross the DLL boundary: Rust-allocated objects/strings are freed by a Rust export, and Harbour-allocated memory is freed by Harbour.

## FFI safety contract

Mandatory rules:

1. **Opaque ownership:** Harbour holds a handle; only Rust creates/destroys Rust objects.
2. **No unwind:** every exported function has a panic guard and returns a status code.
3. **Strings:** input is a UTF-8 pointer + byte length; Rust copies it if it must outlive the call. There is no NUL assumption.
4. **Memory symmetry:** memory allocated by Rust is freed only with Rust `rtui_free_*`; caller-provided buffers are even better.
5. **Thread affinity:** a session handle is used only by the thread that created it unless explicitly designed and tested otherwise.
6. **Callbacks:** synchronous, without retaining temporary pointers after the callback and without exceptions crossing the C boundary.
7. **Terminal restore:** idempotent `close`; cleanup after a partially failed `open`; defined behavior on Harbour error/quit.
8. **Versioning:** `rtui_abi_version()`, `struct_size`, and feature query. Ratatui remains a hidden implementation detail.
9. **Limits:** validate coordinates/dimensions (`u16` in Ratatui), array lengths, UTF-8, and enum values before calling the Rust API.
10. **No borrowed frame:** no `Frame`, `Buffer`, `Cell`, `&str`, or Rust iterator is returned directly to Harbour.

## Proposed MVP

The first binding version should cover:

- session: create/open, close/restore, terminal size, clear;
- frame: begin/end or a single `render(command_list)` function;
- layout: horizontal/vertical constraints and `Rect`;
- style: default, named/ANSI/indexed/RGB colors, modifiers;
- text: UTF-8 text, spans/lines, alignment, wrapping;
- widgets: `Block`, `Paragraph`, `List`, `Table`, `Tabs`, `Gauge`, `Scrollbar`;
- state: selected row/item and scroll position as simple integers/DTOs;
- input: key, mouse, resize, focus/paste only when Rust-owned Crossterm is selected;
- errors: status + copied last-error text;
- testing: off-screen rendering and golden snapshots.

Defer to a later phase:

- `Canvas`, arbitrary closures, and custom widget callbacks;
- experimental `WidgetRef` APIs;
- multiple terminal backends in one build;
- generic third-party widget loading;
- directly exporting serialized internal Ratatui types.

Example low-level C ABI shape:

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

The exact signatures should be fixed after a small prototype; the example above illustrates the ABI shape, not the final interface.

## Risks and mitigations

| Risk | Effect | Mitigation |
|---|---|---|
| Ratatui breaking changes | frequent changes to the wrapper implementation | pin an exact version; keep our C ABI independent; upgrade tests |
| Two Crossterm versions in the community fork | separate event queues/raw-mode state | pin Crossterm 0.29; `cargo tree` CI assertion; one terminal owner |
| Rust/Harbour linker incompatibility | build failure, especially on Windows | CI per toolchain; `cdylib` MVP; separate GNU/MSVC artifacts |
| Two owners of terminal state | corrupted raw mode/alternate screen/input | one session owner; explicit open/close contract |
| Panic across FFI | abort/undefined behavior depending on the ABI scenario | `catch_unwind`, status codes, panic-free boundary |
| Unicode/code-page mismatch | broken glyphs/width | UTF-8 ABI; conversion only at the Harbour edge; tests with wide/combining graphemes |
| Callback lifetime/threading | use-after-free or VM corruption | opaque context, same-thread synchronous callbacks, unregister-before-free |
| 1:1 API explosion | unmaintainable binding surface | curated Harbour API and command DTOs |
| Transitive dependency licenses | packaging compliance | license inventory at release time; preserve notices |

## License

Ratatui is licensed under the MIT License. It permits use, copying, modification, distribution, sublicensing, and sale, provided that the copyright and permission notice are retained in copies/substantial portions: [official LICENSE](https://github.com/ratatui/ratatui/blob/main/LICENSE).

This is compatible with a proprietary or open-source Harbour application. A binary release should nevertheless include the Ratatui MIT notice and an automated license audit for all transitive Cargo dependencies; the Ratatui license alone does not attest to the licenses of the entire dependency tree.

## Decision and next validation

**Go for a prototype.** There is no architectural or licensing blocker, and `ratatui-ffi` already validates the C ABI approach and provides much of the desired surface area. The main choices are whether to fork it or use a smaller adapter, and who owns terminal I/O:

- for a quick standalone Harbour TUI: Rust-owned Crossterm;
- for integration with an existing Harbour GT/TrueColor stack: `HarbourBackend` callbacks or an off-screen buffer.

Recommended technical spike:

1. a short security/API audit of `ratatui-ffi`, followed by a fork or thin wrapper pinned to a specific commit, `ratatui = "=0.30.2"`, and a single `crossterm = "0.29"`;
2. a `cdylib` + C header with 8–12 exported functions;
3. a Harbour demo with `Block` + `Paragraph`, RGB style, Unicode, resize, and clean restoration;
4. an off-screen test that checks the exact cells;
5. Windows x64 and Linux x64 builds; verify the Harbour compiler/linker variant before static linking;
6. after the spike, choose between a command-list API and specific widget functions.

Acceptance criteria for the spike:

- the terminal is restored after normal exit and a recoverable error;
- 1,000 frame cycles without leaks/use-after-free according to sanitizers or a Valgrind equivalent;
- UTF-8, RGB/indexed colors, and wide-glyph tests pass;
- resize does not corrupt the buffer/state;
- the C ABI header contains no Rust-specific types;
- the same Harbour-facing API works with at least one Windows and one Unix artifact.

## Primary sources

- Ratatui repository and README: <https://github.com/ratatui/ratatui>
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
