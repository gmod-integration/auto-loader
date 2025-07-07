# Build stage with Rust 1.88
FROM rust:1.88-bullseye AS builder

# System dependencies for Linux32/64 and Windows32/64 cross-compilation
RUN dpkg --add-architecture i386 \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential \
      gcc-multilib g++-multilib \
      gcc-mingw-w64-i686 g++-mingw-w64-i686 \
      gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 \
      liblua5.1-0-dev \
      curl git pkg-config musl-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build
# Take into account your rust-toolchain.toml for nightly, etc.
COPY rust-toolchain.toml .
COPY . .

RUN echo "→ build Linux 32-bits" \
 && cargo build --release --target i686-unknown-linux-gnu

RUN echo "→ build Linux 64-bits" \
 && cargo build --release --target x86_64-unknown-linux-gnu

RUN echo "→ build Windows 32-bits" \
 && cargo build --release --target i686-pc-windows-gnu

RUN echo "→ build Windows 64-bits" \
 && cargo build --release --target x86_64-pc-windows-gnu

# Artifacts extraction stage
FROM debian:bullseye-slim AS runtime

RUN mkdir -p /out

# Copy compiled artifacts
# Linux 32-bits
COPY --from=builder /build/target/i686-unknown-linux-gnu/release/libgmod_integration_loader.so \
                     /out/gmsv_gmod_integration_loader_linux.dll
COPY --from=builder /build/target/i686-unknown-linux-gnu/release/libgmod_integration.so \
                     /out/gmsv_gmod_integration_linux.dll

# Linux 64-bits
COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/libgmod_integration_loader.so \
                     /out/gmsv_gmod_integration_loader_linux64.dll
COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/libgmod_integration.so \
                     /out/gmsv_gmod_integration_linux64.dll

# Windows 32-bits
COPY --from=builder /build/target/i686-pc-windows-gnu/release/gmod_integration_loader.dll \
                     /out/gmsv_gmod_integration_loader_win32.dll
COPY --from=builder /build/target/i686-pc-windows-gnu/release/gmod_integration.dll \
                     /out/gmsv_gmod_integration_win32.dll

# Windows 64-bits
COPY --from=builder /build/target/x86_64-pc-windows-gnu/release/gmod_integration_loader.dll \
                     /out/gmsv_gmod_integration_loader_win64.dll
COPY --from=builder /build/target/x86_64-pc-windows-gnu/release/gmod_integration.dll \
                     /out/gmsv_gmod_integration_win64.dll

# Use to extract artifacts
CMD ["true"]
