# hapi-rs changelog

648e42e (HEAD -> dev) Cleanup
dad1f61 Adding Async attr access (not working yet)
44dfada Set unique attrib value for NumericAttr
745bbbb Set unique attrib value for i32
37d49af Set unique attrib value for string
66de225 Update to Houdini 20.5.445
95e456f benchmark.hda
b040068 Add memory shared server
6c26968 Add memory shared server
637747c Add memory shared server
779ca99 Debugging presets
d244bec Add more missing APIs
0b5d9a9 Experiment with env_variables
8cdf3e4 BIG change: Wrapper structs are now tuples!
74d1384 Add compositor options
08cf10a Update examples
dfc9b94 Tests pass
adec07d Generate bindings with 20.5.370
d09a0f7 Update all deps
bb615c1 Fix blocking PDG cooking with results
6b18a3e Fixing PDG blocking cook
4ff0765 Fix upcomin Rust 2024 edition warnings
2338484 (tag: back-on-track) Regenerate bindings with struct namke prefix fixed
a0bb206 Cleanup
d86eb95 Add a test for get_cook_result_string
f973c3c Add HAPI_GetMessageNodeIds and HAPI_GetNodeCookResult APIs
02b6099 Attrib dict issue hython example for SESI
5f191c7 Fixed dict attribute count vs totalArrayElements issue
8988dbd Fixed dict attribute count vs totalArrayElements issue
5c96a65 Improving attribute array handling
648e42e (HEAD -> dev) Cleanup
dad1f61 Adding Async attr access (not working yet)
44dfada Set unique attrib value for NumericAttr
745bbbb Set unique attrib value for i32
37d49af Set unique attrib value for string
66de225 Update to Houdini 20.5.445
95e456f benchmark.hda
b040068 Add memory shared server
6c26968 Add memory shared server
637747c Add memory shared server
779ca99 Debugging presets
d244bec Add more missing APIs
0b5d9a9 Experiment with env_variables
8cdf3e4 BIG change: Wrapper structs are now tuples!
74d1384 Add compositor options
08cf10a Update examples
dfc9b94 Tests pass
adec07d Generate bindings with 20.5.370
d09a0f7 Update all deps
bb615c1 Fix blocking PDG cooking with results
6b18a3e Fixing PDG blocking cook
4ff0765 Fix upcomin Rust 2024 edition warnings
2338484 (tag: back-on-track) Regenerate bindings with struct namke prefix fixed
a0bb206 Cleanup
d86eb95 Add a test for get_cook_result_string
f973c3c Add HAPI_GetMessageNodeIds and HAPI_GetNodeCookResult APIs
02b6099 Attrib dict issue hython example for SESI
5f191c7 Fixed dict attribute count vs totalArrayElements issue
8988dbd Fixed dict attribute count vs totalArrayElements issue
5c96a65 Improving attribute array handling
1b0dd50 Add get_attribute_dictionary_array_data
f98bdef Pre-release cleanup 1
b8264ab Geometry attributes more asserts and cleanup
6aad7fd Cleanup
c8459ee Wip on setting dictionary attribute type
adc20a2 Set attribute dictionary type
4866a9d Reading single dict attribute value
e56f275 Add geometry attribute storage test
3cc196b Cleanup
4bdbea0 Revert "Add server pid to the session struct"
3ea82c2 Using threadlocal in tests
00daec5 Add server pid to the session struct
0a6bc64 Add server pid to the session struct

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
