# hapi-rs changelog

## [22.0.0]

- Regenerate bindings for Houdini 22.0.
- Replace dynamically downcast geometry attributes with the exhaustive `AnyAttribute` enum and checked typed
  numeric, string, and dictionary lookups.
- Introduce `Attribute<T, Fixed>` for ordinary fixed-tuple numeric attributes and `Attribute<T, Jagged>` for
  HAPI numeric array attributes, with corresponding shaped string and dictionary handles.
- Rename jagged attribute containers to `JaggedArrayData` and `StringJaggedArrayData`; validate negative sizes,
  checked totals, and flattened data lengths before calling HAPI.
- Bind attribute handles to their node, part, owner, and name. Attribute data methods no longer accept a repeated
  part ID and operate on the complete attribute selected during lookup or creation.
- Derive numeric HAPI storage from the Rust primitive and `Fixed`/`Jagged` shape during creation, and validate all
  fixed and jagged write lengths in release builds.
- Add `async-cooking` extension traits and owning `AsyncJob` handles for asynchronous attribute reads, writes,
  unique-value writes, and indexed string writes.
- Remove the old trait-object/downcasting API, `AttribValueType`, `DataArray`, and compatibility aliases.
- Make session teardown exactly-once across cloned handles, add idempotent `Session::close`, and roll back
  uninitialized native sessions and failed managed HARS startup transactions.
- Track managed versus borrowed HARS processes, terminate and reap managed children, and never remove borrowed
  pipe endpoints.
- Keep potentially expensive HAPI cleanup opt-in, restore the official 100 MB shared-memory default, validate
  public server options, merge server environment settings, and use deadline-based connection retries.

## [21.0.2]
- Regenerate with Houdini 21.0.700
- Use xtask for internal tools
- Fixed most clippy pedantic warnings

## [21.0.1]
- Regenerate bindings with Houdini 21.0.512
- New server architecture - Introduced a server module with support for multiple transport options (shared memory, pipes, sockets) and license preference via `LicensePreference` enum.
- Session initialization refactor - Replaced the old `quick_session` pattern with a new two-step initialization flow (`UninitializedSession` → `initialize()`) and added `new_thrift_session` and `simple_session` convenience methods
- Various session, asset, geometry, attribute and node API updates and improved error handling.
- Fix an issue with passing bitflags to FFI as Rust doesn't support them natively. Intorduced the `ToNodeTypeBits` and `ToNodeFlagBits` to support this.

## [21.0.0]
- Regenerate bindings with Houdini 21.0.440
- Fix async attribute access error by increasing connection count # in `SessionInfo`
- Add `hip_loading.rs` example demonstrating how to save and load Hip files
- Upgrade apps to Rust Edition 2024
- Update dependencies
- Test modules cleanup


## [0.12.0]

- Upgrade to Rust Edition 2024
- Regenerate with Houdini 20.5.445
- Improve error handling when creating Houdini nodes in threaded mode
- Clippy suggestions

## [0.11.2]

- Can now pass a log file to server process.
- Better CookResult enum variant names with `CookErrors` variant containing the error message.
- Add `node_errors.rs` example showing how to retrieve node cooking errors.

## [0.11.1]

- This release signifies a 100% coverage of the C API.
- Async attribute APIs are now fully implemented, but usability can still be improved.
- String attribute APIs put `String-CString` conversion responsibility onto the user.
- Many other API cleanups

## [0.11.0]

- Update to Houdini 20.5.445
- (new in 20.5) Add APIs for setting/getting of unique attribute values
- (new in 20.5) Add initial support for async attribute access (new in 20.5). WIP and not working properly yet.
- (new in 20.5) Add new shared-memory HARS server type
- (new in 20.5) Add HAPI_GetMessageNodeIds and HAPI_GetNodeCookResult APIs
- (new in 20.5) Add performance monitor APIs
- Fixed some issue with PDG blocking cooking
- `quick_session` now uses shared-memory server type instead of named-pipe.
- An experimental Bevy example app.
- Bunch os other small improvements and cleanups
- Mark `Attribute: Send` (by @BerKai97)

**Some public API have been changed (both on the SideFX and this library side)**

## [0.10.0]

- **Minimal** Houdini version bumped to 20.0.625.
- Support new attribute APIs and add some previously missing APIs.
- **Serveral (minimal) public APIs changed**.
- Other fixes and cleanup

## [0.9.3]

- Bump Houdini version to `19.5.716`.
- Add API for working with parameter tags.
- String parameters of type Node can take `NodeHandle` values.
- More PDG WorkItem APIs.
- Expose cache APIs.

## [0.9.2]

### New

- Add `NumericAttribute::read_into()` method for reusing a buffer when reading attribute data.
- Reintroduced an internal reentrant mutex to make sure 2 or more API calls are atomic.
- Add a demo of OpenGL viewer
- Update dependencies

## [0.9.1]

### Changed:

- Remove internal Mutex from `Session`.
- Slightly improved `Parameter` APIs.
- Use `StringHandle` instead of `i32`.
- Update examples.
- Minor cleanups and improvements across the crate.

## [0.9.0]

### New

- New builder pattern for creating nodes.
- New `start_houdini_server` function will launch Houdini application
  with engine server running in it. See _live_session.rs_ example.
- Add many missing library functions.
- Find parameter with tags: `Node::parameter_with_tag`
- Interactive GUI app example that renders the COP network and displays as image.
- Assets can be loaded from memory
- Can now revert parameter values to default.
- Can now remove parameter expressions.
- New `Parameter::save_parm_file` to save the file from the parameter to disk.
- Can now delete geometry attributes.

### Changed

- Simplified `Session::create_node` only take node name now. Use builder pattern for
  more options.
- `connect_to_pipe` now take an optional timeout parameter, and will try to connect multiple times
  until connected or timeout runs out.
- `State` enum renamed to `SessionState`
- Move all tests from modules to _/tests/.._
- Improved image extraction APIs.
- Add support for creating nodes for more asset types with less boilerplate code.
- Fixed setting of array geometry attributes.

## [0.8.0]

### Changed

- AssetLibrary::try_create_first() can now crate nodes other than of Object type.
- Functions taking optional parent (`Option<NodeHandle>`) are now generic and can take `HoudiniNode` too.
- Improve the error type handling and printing.
- Remove `CookOptions` arg from `HoudiniNode::cook`, instead there's new `HoudiniNode::cook_with_options`.
- Add lots of `debug_assert!` for input validation.

### New

- `ManagerType` enum represents a network root node.
- Add several missing geometry APIs.

## [0.7.0]

## Changed

- Reworked parameter APIs
    - Separate `get/set` and `get_array/set_array` methods
    - `get/set` now take an index of a parameter tuple.
    - Eliminate extra String clone for `set_*` string parameters.

## [0.6.0]

### New

- Switch to HAPI-5.0 (Houdini 19.5) API.
- Implement most common PDG APIs for event-based (async) cooking.
- Builder pattern for `SessionOption`
- Example of event-based cooking of PDG network.

### Changed

- Make `DataArray` types public.
- Session creation APIs now require `SessionOptions` argument.
- Add new metadata to `Session` handle with extra information about connection.
- Improve the error type, it now has better error reporting for some APIs.
- Lots of improvements and cleanups.
- Improve build instructions and API documentation.
- More unit tests.
