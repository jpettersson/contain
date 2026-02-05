## Contain

A CLI tool that transparently runs development commands inside Docker containers. Acts as a proxy: you run `contain <command>` and it handles image management, container lifecycle, directory mounting, and file permissions.

### Features

- Configuration via `.contain.yaml` (auto-discovers upward in directory tree)
- Automatic image download from registry or build from Dockerfile
- Proper file permissions via UID/GID injection
- Background containers with `up`/`down`/`status` commands
- Environment variables, ports, and custom mounts

### Usage

#### Running commands

```bash
# Run a command in the container
contain run <command> [args...]

# Interactive mode (keeps STDIN open)
contain run -i <command>

# Open a shell in the container
contain shell
```

#### Background containers

For long-running services or when you want to avoid container startup overhead:

```bash
# Start container in the background
contain up

# Check container status
contain status

# Stop and remove container
contain down
```

When a background container is running, `contain run` executes commands inside it.

#### Options

```bash
# Dry run - show Docker command without executing
contain --dry run <command>

# Pass environment variables
contain -e VAR=value run <command>
contain -e VAR1=value1 -e VAR2=value2 run <command>

# Run as root user
contain --root run <command>

# Keep container after execution (don't auto-remove)
contain -k run <command>

# Skip port mappings from config
contain --skip-ports run <command>
```

### Configuration

Create a `.contain.yaml` file in your project root:

```yaml
images:
  - image: "my-image:latest"
    dockerfile: Dockerfile
    commands: any
```

This minimal configuration tells contain to use `my-image:latest` for any command, building it from `Dockerfile` if not available.

#### Configuration with more options

```yaml
images:
  - image: "my-dev-image:latest"
    name: my-dev-container
    dockerfile: Dockerfile
    commands: any
    env:
      - NODE_ENV=development
    ports:
      - "3000:3000"
    mounts:
      - type: bind
        src: $HOME/.config
        dst: /home/dev/.config
```

The `name` field enables background container support (`contain up`/`down`/`status`).

### Installation

#### Arch Linux

Available on the AUR: [contain](https://aur.archlinux.org/packages/contain/)

#### From source

Requires Rust 1.85+ (edition 2024).

```bash
# Clone and install
git clone <repo-url>
cd contain
cargo install --path .
```

This installs the binary to `~/.cargo/bin/`. Make sure this directory is in your `$PATH`.

### Development

#### Prerequisites

- Rust 1.85+ (for edition 2024 support)
- Docker (for running containerized commands)

#### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

The binary will be at `target/debug/contain` or `target/release/contain`.

#### Running locally

```bash
# Run directly
cargo run -- <command> [args]

# Or use the built binary
./target/debug/contain <command> [args]

# Example: dry run to see the generated Docker command
./target/debug/contain --dry run echo hello
```

#### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

Note: Some integration tests require Docker. If running inside a container, set `CONTAIN_PASSTHROUGH=0` to disable passthrough mode during testing.

#### Installing locally

```bash
# Install to ~/.cargo/bin/
cargo install --path .

# Reinstall after changes
cargo install --path . --force
```
