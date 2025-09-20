FROM rust:1.90.0-bookworm AS build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /usr/src
COPY . /usr/src/webhook-sender/
WORKDIR /usr/src/webhook-sender
RUN cargo build --release && cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS

FROM debian:bookworm-slim

RUN apt-get update; \
	apt-get install -y --no-install-recommends \
		libssl3 ca-certificates; \
	apt-get clean;

WORKDIR /

COPY --chown=root:root --from=build-env \
	/usr/src/webhook-sender/CREDITS \
	/usr/src/webhook-sender/LICENSE \
	/usr/share/licenses/webhook-sender/

COPY --chown=root:root --from=build-env \
	/usr/src/webhook-sender/target/release/webhook-sender \
	/usr/bin/webhook-sender

CMD ["/usr/bin/webhook-sender"]
