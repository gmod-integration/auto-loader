# Use Ubuntu 18.04 for GLIBC 2.27 compatibility and static linking
ARG BASE_IMAGE=ubuntu:18.04
FROM ${BASE_IMAGE} as builder

# Install build dependencies including musl for static linking
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    gcc-multilib \
    g++-multilib \
    pkg-config \
    libssl-dev \
    mingw-w64 \
    musl-tools \
    musl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust with musl targets
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Add target architectures including musl for static linking
RUN rustup target add i686-unknown-linux-gnu
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup target add i686-unknown-linux-musl
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add i686-pc-windows-gnu
RUN rustup target add x86_64-pc-windows-gnu

# Set up cross-compilation environment with static linking
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV CC_i686_unknown_linux_gnu=gcc
ENV CC_x86_64_unknown_linux_gnu=gcc
ENV CC_i686_unknown_linux_musl=musl-gcc
ENV CC_x86_64_unknown_linux_musl=musl-gcc
ENV CC_i686_pc_windows_gnu=i686-w64-mingw32-gcc
ENV CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc

# Static linking flags for musl
ENV RUSTFLAGS_i686_unknown_linux_musl="-C target-feature=+crt-static"
ENV RUSTFLAGS_x86_64_unknown_linux_musl="-C target-feature=+crt-static"

# Windows static linking flags
ENV RUSTFLAGS_i686_pc_windows_gnu="-C target-feature=+crt-static"
ENV RUSTFLAGS_x86_64_pc_windows_gnu="-C target-feature=+crt-static"

WORKDIR /workspace
COPY . .

# Build with static linking - use musl for Linux targets for full static linking
RUN cargo build --release --target i686-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN cargo build --release --target i686-pc-windows-gnu
RUN cargo build --release --target x86_64-pc-windows-gnu

# Verify static linking worked (optional verification step)
RUN echo "Checking Linux 64-bit binary dependencies:" && \
    ldd target/x86_64-unknown-linux-musl/release/libgmod_integration.so || echo "Statically linked (no dependencies - good!)"

# Create artifacts stage
FROM scratch as artifacts

# Copy built libraries with correct naming (musl targets for static linking)
COPY --from=builder /workspace/target/i686-unknown-linux-musl/release/libgmod_integration.so /out/gmsv_gmod_integration_linux.dll
COPY --from=builder /workspace/target/x86_64-unknown-linux-musl/release/libgmod_integration.so /out/gmsv_gmod_integration_linux64.dll
COPY --from=builder /workspace/target/i686-pc-windows-gnu/release/gmod_integration.dll /out/gmsv_gmod_integration_win32.dll
COPY --from=builder /workspace/target/x86_64-pc-windows-gnu/release/gmod_integration.dll /out/gmsv_gmod_integration_win64.dll

COPY --from=builder /workspace/target/i686-unknown-linux-musl/release/libgmod_integration_loader.so /out/gmsv_gmod_integration_loader_linux.dll
COPY --from=builder /workspace/target/x86_64-unknown-linux-musl/release/libgmod_integration_loader.so /out/gmsv_gmod_integration_loader_linux64.dll
COPY --from=builder /workspace/target/i686-pc-windows-gnu/release/gmod_integration_loader.dll /out/gmsv_gmod_integration_loader_win32.dll
COPY --from=builder /workspace/target/x86_64-pc-windows-gnu/release/gmod_integration_loader.dll /out/gmsv_gmod_integration_loader_win64.dll