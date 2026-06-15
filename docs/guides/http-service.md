# Run the HTTP service

A small axum server for declension lookups — the container deployment.

## Run

```sh
cargo run -p keinontolibrary-server         # listens on 0.0.0.0:8080
```

Configuration is via environment:

| var | default | purpose |
| --- | --- | --- |
| `KEINONTO_ARTIFACT` | `data/artifact/keinontolibrary.bin` | the packed artifact |
| `KEINONTO_OVERLAY` | `data/overlay.jsonl` | persistent overlay (admin writes) |
| `KEINONTO_ADDR` | `0.0.0.0:8080` | bind address |
| `KEINONTO_ADMIN_TOKEN` | _(unset)_ | bearer token; admin endpoints are **disabled** unless set |
| `RUST_LOG` | `info` | log level (structured tracing; requests are traced) |

## Endpoints

```sh
curl 'localhost:8080/decline?word=hevonen&number=plural&case=inessive'
# {"variants":["hevosissa"],"status":"present","source":"lookup","coincides_with":null}

curl 'localhost:8080/paradigm?word=talo'          # full table as JSON
curl 'localhost:8080/healthz'                      # "ok"
curl 'localhost:8080/about'                        # version, data metadata, attribution
```

Both `/decline` and `/paradigm` accept `&hn=` and `&tn=` to disambiguate homonyms.

Response status codes mirror the engine: `200` ok, `400` bad number/case (or overlong
`word`), `404` unknown word, `409` ambiguous (body lists the candidate paradigms), `422`
defective form.

## Admin (overlay mutation)

Enabled only when `KEINONTO_ADMIN_TOKEN` is set. Both paths are aliases (create-or-replace):

```sh
curl -X POST localhost:8080/admin/add \
  -H "authorization: Bearer $KEINONTO_ADMIN_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"lemma":"uudissana","tn":9,"number":"singular","case":"inessive","variants":["uudissanassa"]}'
```

The token is compared in constant time (SHA-256 digests); bad tokens get `403`. Request
bodies are capped at 16 KiB. Put the service behind a proxy for TLS and rate limiting.

## Container

```sh
cargo run -p keinontolibrary-ingest          # produce data/artifact/keinontolibrary.bin first
docker build -t keinontolibrary .            # ~10 MB static-musl scratch image
docker run -p 8080:8080 keinontolibrary
```

The image runs unprivileged (`USER 65532`) and ships a `HEALTHCHECK` (the binary
self-probes via `--health`). The server drains in-flight requests on SIGTERM/SIGINT, so
it stops cleanly under an orchestrator.
