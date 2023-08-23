FROM clux/muslrust:stable AS builder

ADD Cargo.toml /volume
ADD src/ /volume/src

RUN cargo build --release 

FROM scratch

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/lb /

ENTRYPOINT ["/lb"]
