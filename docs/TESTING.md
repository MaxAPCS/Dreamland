# Testing
## Unit Tests
### Continuous Integration
[Github Actions](../.github/workflows/tests.yml),
[wasm-bindgen-test](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/index.html)
<br>
This pipeline compiles and runs unit tests for each push (specified with `#[wasm_bindgen_test]`). Currently implemented are sanity checks and behavior confirmations for the quadtree system, arguably the most opaque and complex undergirder of the project framework. The tests run on a node.js headless environment but undergo the same compilation process as the release build.

### Debug Assertions
[debug_assert! macro](https://doc.rust-lang.org/std/macro.debug_assert.html)
<br>
Less a testing framework and more a reassurance practice, many of the safety checks ensuring abstractions work as I assume they do are implemented as debug assertions. This avoids extra computation in releases, but guarantees correctness in debug builds.

## Higher-Level
### Continuous Deployment
[Github Actions](../.github/workflows/gh-pages.yml) -> [Live Instance](https://ucsb-cmptgcs1l-f25.github.io/max-dreamland/)
<br>
This pipeline compiles each push, creates an artifact, and deploys it to github pages. This ensures the code has no compile-time errors and provides for easy end-to-end testing on multiple devices.
<br>
It took multiple iterations to get cross-compilation succeeding, due to failiures of external WASM linking steps.

### Integration Testing
[Chunk Log Viewer](./chunk_log_viewer.html), [World State Viewer](./world_state_viewer.html)
<br>
These tools, built in HTML/CSS/JS with the help of LLMs, provide an intuitive and clarifying view of the actions of world generation. Formerly, changing the log level in the code to debug would display the standardized logs of the world, quadtree, and compute frameworks interoperating. This was a crucial tool for debugging and understanding the complexities of dynamic chunk and LOD generation, and has been very useful. Changing the log level now displays full world state snapshots, built to be interpreted by the world state viewer. This gives a big-picture view and helped quash a few fine-grained bugs in the system. These are both provided here for future reference.