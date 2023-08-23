FROM clux/muslrust:stable AS builder

ADD .cargo Cargo.toml Cargo.lock /volume
ADD .cargo/ /volume/.cargo
ADD src/ /volume/src

RUN cargo build --locked --release 

FROM scratch

COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/rinha /
ENV RUST_LOG=warn,listener=debug
ENV SERVER_ADDRESS=0.0.0.0:80

ENTRYPOINT ["/rinha"]
