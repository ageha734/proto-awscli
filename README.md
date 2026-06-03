# proto-awscli

Proto WASM plugin for [AWS CLI v2](https://aws.amazon.com/cli/).

This plugin enables installing and managing AWS CLI v2 through [proto](https://moonrepo.dev/proto),
the pluggable multi-language version manager.

## Why WASM?

AWS CLI v2 cannot be distributed as a simple binary download. It requires platform-specific
installer scripts (Linux), package files (macOS `.pkg`), or MSI installers (Windows).
This WASM plugin implements the `native_install` hook to handle these custom installation
procedures.

## Installation

Add the plugin to your `.prototools` file:

```toml
[plugins]
awscli = "github://ageha734/proto-awscli"
```

Then install AWS CLI:

```bash
proto install awscli
# or a specific version
proto install awscli 2.22.0
```

## Supported Platforms

| OS      | Architecture    | Method       |
|---------|----------------|--------------|
| Linux   | x86_64, arm64  | zip + install script |
| macOS   | x86_64, arm64  | pkg extraction |
| Windows | x86_64         | MSI installer |

## Usage

Once installed, the `aws` command is available:

```bash
proto run awscli -- --version
proto run awscli -- s3 ls
```

Or use it directly through proto shims:

```bash
aws --version
aws configure
```

## Version Resolution

Versions are resolved from Git tags on the [aws/aws-cli](https://github.com/aws/aws-cli)
repository. Only v2 tags are included.

Supported version specifications:

- Exact: `2.22.0`
- Range: `>=2.20.0`
- Alias: `latest`, `v2`

## Development

### Prerequisites

- Rust toolchain with `wasm32-wasip1` target
- proto (for testing)

### Build

```bash
cargo build --target wasm32-wasip1 --release
```

### Test

```bash
cargo test
```

### Local testing with proto

```bash
cargo build --target wasm32-wasip1 --release
proto plugin add awscli source:./target/wasm32-wasip1/release/proto_awscli.wasm
proto install awscli latest
aws --version
```

## License

MIT
