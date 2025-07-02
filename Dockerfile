# --- Étape 1 : build avec rust:1.88 + nightly via rust-toolchain.toml ---
FROM rust:1.88 AS builder

# Prérequis systèmes pour Linux32/64 et Windows32/64
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
COPY rust-toolchain.toml .
COPY . .

# Compilation du workspace pour chaque target
RUN echo "→ build Linux 32-bits" \
 && cargo build --release --target i686-unknown-linux-gnu \
 \
 && echo "→ build Linux 64-bits" \
 && cargo build --release --target x86_64-unknown-linux-gnu \
 \
 && echo "→ build Windows 32-bits" \
 && cargo build --release --target i686-pc-windows-gnu \
 \
 && echo "→ build Windows 64-bits" \
 && cargo build --release --target x86_64-pc-windows-gnu

# --- Étape 2 : extraction des artefacts finaux ---
FROM scratch AS artifacts

# Linux 32-bits
COPY --from=builder /build/target/i686-unknown-linux-gnu/release/libgmod_integration_loader.so \
                     /out/gmsv_gmod_integration_loader_linux.dll
COPY --from=builder /build/target/i686-unknown-linux-gnu/release/libgmod_integration.so \
                     /out/gmod_integration_linux.dll

# Linux 64-bits
COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/libgmod_integration_loader.so \
                     /out/gmsv_gmod_integration_loader_linux64.dll
COPY --from=builder /build/target/x86_64-unknown-linux-gnu/release/libgmod_integration.so \
                     /out/gmod_integration_linux64.dll

# Windows 32-bits
COPY --from=builder /build/target/i686-pc-windows-gnu/release/gmod_integration_loader.dll \
                     /out/gmsv_gmod_integration_loader_win32.dll
COPY --from=builder /build/target/i686-pc-windows-gnu/release/gmod_integration.dll \
                     /out/gmod_integration_win32.dll

# Windows 64-bits
COPY --from=builder /build/target/x86_64-pc-windows-gnu/release/gmod_integration_loader.dll \
                     /out/gmsv_gmod_integration_loader_win64.dll
COPY --from=builder /build/target/x86_64-pc-windows-gnu/release/gmod_integration.dll \
                     /out/gmod_integration_win64.dll

CMD ["true"]