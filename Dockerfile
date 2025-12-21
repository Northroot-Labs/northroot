# Multi-stage build for slim devcontainer
FROM rust:1.91-slim as base

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    ca-certificates \
    ripgrep \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

# Install just (command runner)
RUN curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin

# Install Rust nightly for miri
RUN rustup toolchain install nightly --component miri --profile minimal

# Install cargo tools (grouped for better caching)
RUN cargo install --locked \
    cargo-llvm-cov \
    cargo-deny \
    cargo-audit \
    cargo-fuzz

# Install rust-analyzer (LSP)
RUN curl -L https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz | \
    gunzip -c - > /usr/local/bin/rust-analyzer && \
    chmod +x /usr/local/bin/rust-analyzer

# Use shared roots so non-root user can use toolchain
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:/usr/local/bin:${PATH}"

# Create non-root user (defaults can be overridden via build args)
ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=${USER_UID}
RUN groupadd --gid ${USER_GID} ${USERNAME} && \
    useradd --uid ${USER_UID} --gid ${USER_GID} --create-home ${USERNAME} && \
    chown -R ${USERNAME}:${USER_GID} /usr/local/rustup /usr/local/cargo

# Set environment variables
ENV CARGO_TERM_COLOR=always
ENV RUSTFLAGS=-Dwarnings

# Switch to non-root user
USER ${USERNAME}

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]

