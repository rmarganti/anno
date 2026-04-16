# MUST dos

- **IMPORTANT**: before you do anything else, run the `beans prime` command and heed its output.
- All commit messages should following conventional commits. Examples:
    - `feat: added ability to do thing`
    - `fix: fixed some bug`
    - `docs: updated README.md with build info`

## Verifying (MUST BE RUN BEFORE CONSIDERING A TASK COMPLETE)

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --all-features`
