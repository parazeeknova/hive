FROM debian:trixie-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
RUN apt-get update && apt-get install -y --no-install-recommends squid ca-certificates && rm -rf /var/lib/apt/lists/*
COPY squid.conf /etc/squid/squid.conf
USER proxy
ENTRYPOINT ["squid", "-N", "-f", "/etc/squid/squid.conf"]
