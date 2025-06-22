FROM ubuntu:24.04 AS builder
WORKDIR /root
COPY . /root
RUN apt update && \
    apt install curl gcc -y && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    ~/.cargo/bin/cargo build --release

FROM gcr.io/distroless/cc
EXPOSE 4567/tcp
WORKDIR /
COPY --from=builder /root/target/release/echolite .
CMD ["./echolite"]
