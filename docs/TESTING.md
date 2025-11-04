Assignment:
```
implement and document at least one unit test and one higher-level (either component or integration or end-to-end) test in your code base.
document the testing library/ies and approach/es you experimented with and the unit and higher-level test(s) that you implemented in your codebase.
```

## Unit Tests
### Rust Tests
// TODO

## Higher-Level
### CI/CD
[Github Actions](../.github/workflows/gh-pages.yml) -> [Live Instance](https://ucsb-cmptgcs1l-f25.github.io/max-dreamland/)
<br>
This pipeline compiles each push, creates an artifact, and deploys it to github pages. This ensures the code has no compile-time errors and provides for easy end-to-end testing on multiple devices.
<br>
It took multiple iterations to get cross-compilation succeeding, due to failiures of external WASM linking steps.