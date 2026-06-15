# Multi-stage build → static musl binary on a scratch image.
#
# Prerequisite: the lookup artifact must exist at data/artifact/keinontolibrary.bin
# (run `cargo run -p keinontolibrary-ingest` first). It is not committed because it embeds
# Voikko-derived data whose redistribution license is unresolved (see LICENSING.md); the
# operator building the image is responsible for that data.

# ---- build stage -----------------------------------------------------------------------
# rust:alpine targets *-unknown-linux-musl by default, so `cargo build` yields a static
# binary with no libc dependency. Pin the exact toolchain for reproducible builds.
FROM rust:1.82-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p keinontolibrary-server

# ---- runtime stage ---------------------------------------------------------------------
FROM scratch
COPY --from=builder /build/target/release/keinontolibrary-server /keinontolibrary-server
COPY data/artifact/keinontolibrary.bin /data/artifact/keinontolibrary.bin

ENV KEINONTO_ARTIFACT=/data/artifact/keinontolibrary.bin \
    KEINONTO_OVERLAY=/data/overlay.jsonl \
    KEINONTO_ADDR=0.0.0.0:8080

EXPOSE 8080
# Run unprivileged: scratch has no users, so use a fixed non-root uid:gid. If admin
# writes are enabled, mount the overlay dir as a volume chowned to this id.
USER 65532:65532

# scratch has no shell or curl; the server self-probes via `--health`, which makes one
# request to /healthz on KEINONTO_ADDR and exits 0 (ok) or 1 (down).
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/keinontolibrary-server", "--health"]

ENTRYPOINT ["/keinontolibrary-server"]
