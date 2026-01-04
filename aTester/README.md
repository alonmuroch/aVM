# aTests

Architecture-aware test suite scaffold.

Intent:
- Run ELF binaries on a supplied architecture runner (VM, QEMU, etc).
- Allow multiple test kinds with different assertion logic.
- Keep the runner abstract and pluggable.

Current shape:
- `ArchRunner`: runs an ELF on an architecture and returns logs/exit code.
- `TestEvaluator`: evaluates a `RunResult` based on `TestCase` kind.
- `Suite`: runs a list of test cases through a runner.
