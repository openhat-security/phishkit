# Ubuntu (not Alpine): Tauri / WebKitGTK cannot build against musl.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl ca-certificates git pkg-config \
    libgtk-3-dev librsvg2-dev patchelf \
    libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
    libsoup-3.0-dev libssl-dev \
    xvfb ffmpeg \
    python3 python3-venv \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable \
    && rustup component add rustfmt clippy

WORKDIR /src
COPY tests/integration/run.sh /usr/local/bin/phishkit-integration
RUN chmod +x /usr/local/bin/phishkit-integration

CMD ["phishkit-integration"]
