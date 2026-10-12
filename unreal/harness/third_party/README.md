# Vendored test-only headers

None of these reaches the plugin or a game: they build the harness's suites
and its EK fixture only (ADR 1.10.26ac D10).

| File | Project | Version | Licence | Taken from |
|---|---|---|---|---|
| `doctest.h` | doctest | 2.4.6 | MIT, with Intel's modifications under Apache-2.0 (its header says so) | oneTBB 2021.5.0's `test/common/doctest.h` |
| `json.hpp` | nlohmann/json | 3.12.0 | MIT | the single-include header |
| `httplib.h`, `httplib.cpp` | cpp-httplib | 0.56.0 | MIT | its split form (`split.py`): the declarations and their implementation |

The fixture uses cpp-httplib for plain HTTP to `127.0.0.1` only, so it needs
no TLS library.
