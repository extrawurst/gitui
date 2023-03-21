# Git Commit Signing Tests

Test commit signing in the isolation of a container.

- Create GPG and SSH keys without side effects
- Call real binaries in shellouts

## Requirements

- Docker

## Usage

### Build the Test Container

*Run in the project root:*

```bash
docker build --tag gitui:sign-test --file Dockerfile.sign-test .
```

The image is around ~1GB.

It can be pushed in CI to leverage layer caching from the registry.

### Run the Tests

```bash
docker run --rm gitui:sign-test cargo test --workspace
```

The `CARGO_HOME` can be cached in CI to a known location in order to save compile time within the container.
See [Caching the Cargo home in CI](https://doc.rust-lang.org/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci)
on the official documentation.
