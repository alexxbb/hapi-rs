# hapi-rs changelog
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

## Changed
- Make `DataArray` types public.
- Session creation APIs now require `SessionOptions` argument.
- Add new metadata to `Session` handle with extra information about connection.
- Improve the error type, it now has better error reporting for some APIs.
- Lots of improvements and cleanups.
- Improve build instructions and API documentation.
- More unit tests.
