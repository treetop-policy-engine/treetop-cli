# syntax=docker/dockerfile:1
FROM docker.io/library/rust:1.98.1-alpine3.24@sha256:7cc1c22d77d9432f7fe012a70e6d3e555af54c2a6832700ed7d553f1769ae89f AS builder

ARG CARGO_BUILD_FLAGS="--locked --release"
ARG TREETOP_CLI_BUILD_CHANNEL="dev"
ARG TREETOP_CLI_BUILD_GIT_SHA=""
ARG TREETOP_CLI_BUILD_TIMESTAMP="unknown"

WORKDIR /usr/src/treetop-cli

# Alpine's native Rust target uses musl. The C toolchain and CMake build the
# statically linked cryptography used by rustls.
RUN apk add --no-cache build-base cmake

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/treetop-cli/target \
    TREETOP_CLI_BUILD_CHANNEL="${TREETOP_CLI_BUILD_CHANNEL}" \
    TREETOP_CLI_BUILD_GIT_SHA="${TREETOP_CLI_BUILD_GIT_SHA}" \
    TREETOP_CLI_BUILD_TIMESTAMP="${TREETOP_CLI_BUILD_TIMESTAMP}" \
    cargo build ${CARGO_BUILD_FLAGS} --bin treetop-cli && \
    cp target/release/treetop-cli /tmp/treetop-cli

RUN /tmp/treetop-cli --version

FROM scratch AS release-artifacts

COPY --from=builder /tmp/treetop-cli /treetop-cli
