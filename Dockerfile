FROM debian:11

RUN apt update && apt install -y gcc-multilib g++-multilib curl git pkg-config musl-dev

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add i686-unknown-linux-gnu

WORKDIR /build
COPY . .

RUN cargo build --target i686-unknown-linux-gnu --release
