# build the project
build:
  cargo build --release --all-features

# run tests
test: 
  cargo test --all-features

# run cargo fmt
format:
    cargo fmt --all

# check format and clippy
check:
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D clippy::all -D warnings
