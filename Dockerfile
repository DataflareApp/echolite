FROM ubuntu:24.04 as builder
WORKDIR /root
COPY . /root
RUN apt update
RUN apt install curl gcc -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN ~/.cargo/bin/cargo build --release

FROM gcr.io/distroless/cc
EXPOSE 4567/tcp
WORKDIR /
COPY --from=builder /root/target/release/echolite .
CMD ["./echolite"]
