FROM debian:bookworm-slim AS download

ARG FALLOW_VERSION=3.24.1
ARG TARGETARCH

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

# The sha256 pins below are bound to FALLOW_VERSION above; bump both together.
# The maintainer release flow refreshes all three after publication, via
# .github/scripts/update-dockerfile-pins.mjs. There is no CI job that does it:
# the docker-lockstep job that used to open a PR here was removed in v3.7.1.
RUN set -eux; \
  case "${TARGETARCH}" in \
    amd64) \
      asset="fallow-linux-x64-musl"; \
      sha256="e5d66acf94483c794ef99ceb03d66b1c530947a8cdde58b05996b61e9f00cbcb"; \
      ;; \
    arm64) \
      asset="fallow-linux-arm64-musl"; \
      sha256="3f6776190cda834c674884b43fdfaf66bab580be20e8ed2a2551852f1d2d9a95"; \
      ;; \
    *) \
      echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; \
      exit 1; \
      ;; \
  esac; \
  curl -fsSL --retry 5 --retry-connrefused --retry-delay 2 \
    "https://github.com/fallow-rs/fallow/releases/download/v${FALLOW_VERSION}/${asset}" -o /usr/local/bin/fallow; \
  echo "${sha256}  /usr/local/bin/fallow" | sha256sum -c -; \
  chmod +x /usr/local/bin/fallow

FROM node:26-bookworm-slim AS runtime

ARG COREPACK_VERSION=0.35.0

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git \
  && npm install -g "corepack@${COREPACK_VERSION}" \
  && corepack enable \
  && npm cache clean --force \
  && rm -rf /var/lib/apt/lists/*

COPY --from=download /usr/local/bin/fallow /usr/local/bin/fallow

WORKDIR /workspace
ENTRYPOINT ["fallow"]
CMD ["--help"]
