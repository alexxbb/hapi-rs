# AGENTS.md

Guidance for AI coding agents working in this repository. For a user-facing
introduction to the crate see `README.md`; this file is intentionally
contributor-oriented and assumes you are editing the code.

## Project overview

`hapi-rs` is a Rust binding crate for **Houdini Engine** (HAPI), the C API
exposed by SideFX Houdini. The crate wraps `HAPI_`* functions and structs in
ergonomic, type-safe Rust while keeping the public surface aligned with the
official C API so users can cross-reference SideFX documentation.

A valid Houdini Engine license and a local Houdini install are required to
build (linker needs `libHAPIL`) and to run anything that touches a live
`Session` (almost all tests and examples).

## Repository layout

This is a Cargo workspace. Members and their roles:

- `lib/` — the published `hapi-rs` crate. **All library code changes happen
here.** Workspace `default-members = ["lib"]`, so plain `cargo` commands at
the root only touch this crate.
  - `lib/src/ffi/` — the raw FFI boundary.
    - `bindings.rs` — `bindgen`-generated, **do not hand-edit**. Regenerated
    via `cargo xtask bindgen`.
    - `functions.rs` — thin, internal-only `unsafe` wrappers around
    `HAPI_`* calls. New raw calls go here.
    - `structs.rs` — Rust newtype wrappers around `HAPI_`* C structs with
    `get_`* / `set_*` / `with_*` accessors generated via macros.
  - `lib/src/{asset,attribute,geometry,material,node,parameter,pdg,session,server,volume,cop,stringhandle}.rs(/)`
  — public modules that roughly mirror the chapters of the HAPI manual.
  - `lib/tests/` — integration tests; every public module has a matching
  file (e.g. `lib/tests/parameters.rs`).
  - `lib/examples/` — runnable examples; treat them as executable docs.
- `xtask/` — workspace helper commands. Includes:
  - `hapi_bindgen` module that regenerates `lib/src/ffi/bindings.rs` from the
  Houdini headers (`cargo xtask bindgen`).
  - `coverage` module that reports which `HAPI_*` symbols are not yet wrapped
  (`cargo xtask coverage`).
- `apps/{viewport,bevy,render_cop}` — standalone demo applications. Not
published; safe to compile, but they pull large dep trees.
- `benchmarks/server/` — micro-benchmarks for session/server startup.
- `otls/` — `.hda` Houdini Digital Assets used by tests and examples.
**Do not modify** binary assets; they are inputs to fixtures.
- `hython/` — Python helper scripts run under SideFX's `hython` for
reproducing engine-side behavior. Not invoked by Rust tests.
- `engine_docs/www.sidefx.com/docs/hengine/` — full offline mirror of the
official HAPI 8.0 docs. See "Understanding the C API" below.
- `.github/workflows/ci.yml` — CI runs `cargo check --all-features` only.
It does **not** run tests (no Houdini install on GitHub runners). Local
testing is the gate.

## Environment & build

Required environment variable:

- `HFS` — path to the installed Houdini (e.g. `/opt/hfs21.0.512`,
`/Applications/Houdini/Current/Frameworks/Houdini.framework/Versions/Current/Resources`,
or `C:\Program Files\Side Effects Software\Houdini X.Y.Z`).

`lib/build.rs` reads `$HFS/toolkit/hdk_api_version.txt` and **panics if its
major.minor does not match `Cargo.toml`'s version**. The crate version is
intentionally locked to the Houdini version it targets (currently
`21.0.x` ↔ Houdini 21.0). If you bump one, bump the other.

Runtime library discovery (needed to actually run, not just to link):

- Linux: `LD_LIBRARY_PATH=$HFS/dsolib` or an rpath via `RUSTFLAGS`.
- macOS: `DYLD_LIBRARY_PATH=…/Libraries` or rpath.
- Windows: prepend `$HFS/bin` to `PATH`.

CI sets `HFS=""` and only runs `cargo check`; the `build.rs` short-circuits
when `CI` is set. Do not add tests that would need to run on CI.

## Common commands

Run everything from the workspace root unless noted.

- `cargo check` — fast type-check of `lib/`.
- `cargo check --workspace --all-features` — type-check everything including
`apps/` and `benchmarks/`.
- `cargo test` — run integration tests in `lib/tests/` (requires a working
`HFS` and a license). Tests share a thread-local `Session` via
`lib/tests/utils.rs`; prefer `with_session(|s| { ... })` in new tests.
- `cargo test --test parameters` — run a single integration file.
- `cargo test -- --nocapture` — see `println!` / `env_logger` output.
- `cargo run --example parameters` — run any file in `lib/examples/`.
- `cargo xtask bindgen` — regenerate `lib/src/ffi/bindings.rs` (requires
  `HFS` env var set and reads version from
  `$HFS/toolkit/cmake/HoudiniConfigVersion.cmake`).
- `cargo xtask bindgen -- --outdir <path>` — write generated bindings to a
  different output directory.
- `cargo xtask coverage` — report unwrapped `HAPI_*` symbols.
- `cargo fmt` — format. `rustfmt.toml` sets `reorder_modules = false`;
preserve the existing module order in `lib/src/lib.rs`.
- `cargo clippy --all-targets` — lint. Fix warnings you introduce.

## Understanding the C API (mandatory before touching FFI)

The FFI boundary (`lib/src/ffi/`) and any new public method that wraps a
`HAPI_`* call must be designed against the **official Houdini Engine
documentation**. Before adding or modifying such code:

1. Read the relevant section of the HAPI docs. A complete offline mirror
  lives at `engine_docs/www.sidefx.com/docs/hengine/`; use it instead of
   guessing. The online version is at
   [https://www.sidefx.com/docs/hengine/](https://www.sidefx.com/docs/hengine/).
2. Identify the exact `HAPI_`* function(s) you are wrapping, their argument
  ownership (who allocates strings/arrays, who frees them), threading
   guarantees, and the meaning of every out-parameter.
3. Match the C semantics. Do not silently change defaults, swap argument
  order, or drop error codes. Every `HAPI_Result` must reach the user as a
   `HapiError` via `check_err` (see existing patterns in
   `lib/src/ffi/functions.rs`).
4. Cross-reference an existing wrapper for the same module (e.g. for a new
  geometry call, mimic `geometry.rs`) before inventing a new pattern.

When the C API and a "nicer" Rust shape disagree, document the deviation in
a doc comment that links back to the relevant HAPI page.

## Adding a new public API

Typical flow when exposing a previously-unwrapped `HAPI_`* symbol:

1. Add an `unsafe` wrapper in `lib/src/ffi/functions.rs` that calls the raw
  binding, handles out-params with `MaybeUninit`, and routes errors through
   `check_err(session, || "Calling HAPI_Foo")`.
2. If the call produces or consumes a `HAPI_`* struct, add or extend a
  wrapper in `lib/src/ffi/structs.rs`
  1. Not all struct fields need to have all acessor methods(`set!, get!, with!)`
    `get!` must always be implemented, then carefully look at the C API and do your best judgement on which fields are actually makes sense to expose expose with `set!` and `with!`
3. Expose the operation as a method on the appropriate public type
  (`Session`, `HoudiniNode`, `Geometry`, `Parameter`, …) with a doc comment
   that summarizes the C behavior and links to the HAPI docs page.
4. If re-generating bindings for new version, re-export any new enums via `lib/src/ffi/mod.rs::enums` so users can
  access them without reaching into `raw`.

## Testing rules

- **Every public API must have at least one test.** When you add or
meaningfully change a public function/method, add or extend a test in
`lib/tests/<module>.rs`. A PR that grows the public surface without a
corresponding test should not be considered complete.
- Tests live in `lib/tests/`, **not** in `#[cfg(test)] mod tests` blocks
inside `lib/src/`. This is a deliberate convention; keep it.
- Reuse the shared session helpers from `lib/tests/utils.rs`
(`with_session`, `with_session_asset`, `with_test_geometry`,
`create_triangle`, `create_single_point_geo`). Spinning up a fresh session
per test is only justified when isolation is required (see
`session_get_set_time` for the rationale comment).
- Load HDAs via the `HdaFile` enum so test fixtures stay centralized.
- Tests are not run on CI; assume they must pass locally before review.
- Doc-tests inside `lib/src/lib.rs` and module docs are compiled as part of
`cargo test`; if you change a doc example, run it.

## Coding conventions

- Rust edition **2024**.
- Public types drop the `HAPI_` prefix and use concise variant names but
keep C ordering for enums (so docs cross-reference cleanly).
- Most wrapper structs implement `Default` and offer a builder-style
`with_`* API. Follow that pattern for new structs.
- `Session` is `Clone + Send + Sync` and internally `Arc`-wrapped; do **not**
add a `Mutex` around it. Use the private `ReentrantMutex` already on
`SessionInner` only when HAPI explicitly requires serialized calls.
- Errors: return `crate::Result<T>` (alias for `Result<T, HapiError>`). Add
context with `.context(...)` / `.with_context(...)` from the `errors`
module instead of formatting messages by hand.
- Strings: never expose `HAPI_StringHandle` directly. Route through
`crate::stringhandle` and prefer `StringArray` for batches.
- Logging: use the `log` crate (`debug!`, `error!`). Tests can opt into
output with `env_logger::try_init()`.
- `unsafe` is allowed inside `lib/src/ffi/` and nowhere else without a
comment explaining the invariant.
- Keep the module order in `lib/src/lib.rs` stable (rustfmt is configured
not to reorder it).

## Don't

- Don't hand-edit `lib/src/ffi/bindings.rs`. Regenerate via
`cargo xtask bindgen` when Houdini headers change.
- Don't desync `lib/Cargo.toml`'s version from the supported Houdini
major.minor — `build.rs` will refuse to build.
- Don't modify files in `otls/` or `engine_docs/`; they are vendored
inputs.
- Don't add tests that depend on network access, paid SideFX assets beyond
what's already in `otls/`, or a specific OS unless `#[cfg]`-gated.
- Don't introduce `Mutex<Session>` or any global mutable state in the
public API.
- Don't expand the `raw` re-exports beyond what is strictly necessary;
prefer adding a safe wrapper.

## PR checklist

Before declaring work done:

- `cargo fmt` is clean.
- `cargo clippy --all-targets` is clean for files you touched.
- `cargo check --workspace --all-features` succeeds.
- `cargo test` passes locally (requires Houdini + license).
- Any new or changed public API has at least one test in
`lib/tests/<module>.rs`.
- Any FFI change references the corresponding HAPI doc page in a doc
comment.
- `CHANGELOG.md` updated under the current unreleased version if the
change is user-visible.

