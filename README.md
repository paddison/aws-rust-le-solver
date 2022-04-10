# aws-rust-le-solver

A small Program that creates a lambda function that solves a linear equation system, by reading from a file in a bucket, storing the result in another bucket.

# Prerequisites

Make sure cargo lambda is installed:
```
> cargo lambda --version
> cargo-lambda 0.5.3
```
Otherwise run:
```
cargo install cargo-lambda
```

# Deploy

run `sh ./deploy.sh` from your terminal.

## Note
If your system is not x86_64, change `cargo lambda build --release --target x86_64-unknown-linux-gnu --output-format zip` to `cargo lambda build --release --target aarch64-unknown-linux-gnu --output-format zip` in deploy.sh.

# License

MIT
