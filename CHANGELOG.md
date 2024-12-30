# hapi-rs changelog

## [0.11.0]

- Update to Houdini 20.5.445
- (new in 20.5) Add APIs for setting/getting of unique attribute values
- (new in 20.5) Add initial support for async attribute access (new in 20.5). WIP and not working properly yet.
- (new in 20.5) Add new shared-memory HARS server type
- (new in 20.5) Add HAPI_GetMessageNodeIds and HAPI_GetNodeCookResult APIs
- (new in 20.5) Add performance monitor APIs
- Fixed some issue with PDG blocking cooking
- `quick_session` now uses shared-memory server type instead of named-pipe.
- Bunch os other small improvements and cleanups

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
