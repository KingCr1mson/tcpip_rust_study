FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       iproute2 \
       iputils-ping \
       netcat-openbsd \
       tcpdump \
       procps \
    && rm -rf /var/lib/apt/lists/*
