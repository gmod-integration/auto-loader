# Gmod Integration

Auto-loader and updater system for Garry's Mod integration modules.

## Overview

This project provides a two-stage loading system for Garry's Mod:

1. **Auto Loader** (`gmsv_gmod_integration_loader_*.dll`) - Downloads and loads the latest integration modules
2. **Auto Updater** (`gmod_integration_*.dll`) - Updates the actual Gmod integration addon files

## Architecture

-   **Loader Crate** (`crates/loader/`) - Downloads and delegates to real integration modules
-   **Real Crate** (`crates/real/`) - Downloads and installs the actual Gmod integration addon

## Building

### Using Docker (Recommended)

The project includes a multi-stage Dockerfile that cross-compiles for all supported platforms:

```bash
# Build the Docker image
docker build -t gmod-integration-builder .

# Create a temporary container
docker create --name tmp gmod-integration-builder

# Extract the built DLL files
docker cp tmp:/out ./release

# Clean up the temporary container
docker rm tmp
```

This will create a `release/` directory containing all the compiled DLL files:

-   `gmod_integration_linux.dll` - Linux 32-bit
-   `gmod_integration_linux64.dll` - Linux 64-bit
-   `gmod_integration_win32.dll` - Windows 32-bit
-   `gmod_integration_win64.dll` - Windows 64-bit
-   `gmsv_gmod_integration_loader_linux.dll` - Linux 32-bit Loader
-   `gmsv_gmod_integration_loader_linux64.dll` - Linux 64-bit Loader
-   `gmsv_gmod_integration_loader_win32.dll` - Windows 32-bit Loader
-   `gmsv_gmod_integration_loader_win64.dll` - Windows 64-bit Loader

### Manual Building

Requirements:

-   Rust toolchain (see `rust-toolchain.toml`)
-   Cross-compilation targets
-   MinGW for Windows builds

```bash
# Add required targets
rustup target add i686-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu
rustup target add i686-pc-windows-gnu
rustup target add x86_64-pc-windows-gnu

# Build for specific target
cargo build --release --target i686-unknown-linux-gnu
```

## Installation

1. Download the appropriate DLL files for your platform from the [releases page](../../releases)
2. Place them in your `garrysmod/lua/bin/` directory
3. Load the loader module first: `gmsv_gmod_integration_loader_*.dll`

## Auto-Releases

The project automatically builds and releases new versions when changes are made to:

-   `crates/**` - Source code changes
-   `Cargo.*` - Dependency changes
-   `rust-toolchain.toml` - Rust toolchain changes
-   `Dockerfile` - Build configuration changes

Releases are tagged with timestamp and commit hash (e.g. `v20250703-143052-a1b2c3d`).

## Development

### Local Development Script

Use the provided `deploy.sh` script for local development:

```bash
./deploy.sh
```

This will:

1. Build both crates for the target platform
2. Deploy to your development server
3. Increment the version number

### File Structure

```
├── crates/
│   ├── loader/          # Auto-loader module
│   └── real/            # Auto-updater module
├── release/             # Pre-built DLL files
├── .github/workflows/   # CI/CD configuration
├── Dockerfile           # Multi-platform build configuration
├── deploy.sh           # Development deployment script
└── rust-toolchain.toml # Rust version specification
```

## How It Works

1. **Initial Load**: Gmod loads `gmsv_gmod_integration_loader_*.dll`
2. **Update Check**: Loader checks GitHub for latest releases
3. **Download**: Downloads required DLL files for current platform
4. **Delegation**: Loads and delegates to `gmod_integration_*.dll`
5. **Addon Update**: Real module downloads and installs the latest addon files
6. **Integration**: Full Gmod integration is now active

## Logging

Both modules provide timestamped logging in the format:

```
| YYYY-MM-DD HH:MM:SS | Gmod Integration | Auto Loader: <message>
| YYYY-MM-DD HH:MM:SS | Gmod Integration | Auto Updater: <message>
```

## License

[Add your license information here]
