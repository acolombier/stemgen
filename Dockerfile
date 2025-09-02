# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM rust:1-slim-bookworm AS builder
ARG TARGETARCH
ENV CARGO_HOME=/root/.cargo
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Etc/UTC
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && \
    apt-get install -y cmake make libutfcpp-dev unzip libavcodec-dev libavformat-dev libavutil-dev pkg-config clang wget libssl-dev
RUN wget -O taglib.zip https://github.com/acolombier/taglib/archive/refs/heads/feat/mp4-ni-stem.zip && \
    unzip taglib.zip && \
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF taglib-feat-mp4-ni-stem && \
    make -j && \
    make install
WORKDIR /build
ADD . .
WORKDIR /build/cli
ARG BUILD_ARGS=--all-features
RUN --mount=type=cache,target=/build/target/release/build \
    --mount=type=cache,target=/build/target/release/deps \
    --mount=type=cache,target=/build/target/release/incremental \
    --mount=type=cache,target=/build/target/CACHEDIR.TAG \
    --mount=type=cache,target=/build/cli/target/release/build \
    --mount=type=cache,target=/build/cli/target/release/deps \
    --mount=type=cache,target=/build/cli/target/release/incremental \
    --mount=type=cache,target=/build/cli/target/CACHEDIR.TAG \
    --mount=type=cache,target=/root/.cargo/ \
    cargo build --release --bins $BUILD_ARGS
RUN --mount=type=cache,target=/build/target/release/build \
    --mount=type=cache,target=/build/target/release/deps \
    --mount=type=cache,target=/build/target/release/incremental \
    --mount=type=cache,target=/build/target/CACHEDIR.TAG \
    --mount=type=cache,target=/build/cli/target/release/build \
    --mount=type=cache,target=/build/cli/target/release/deps \
    --mount=type=cache,target=/build/cli/target/release/incremental \
    --mount=type=cache,target=/build/cli/target/CACHEDIR.TAG \
    --mount=type=cache,target=/root/.cargo/ \
    cargo test --release --workspace $BUILD_ARGS

FROM --platform=$BUILDPLATFORM debian:bookworm-slim
ARG TARGETARCH
ARG BUILD_ARGS=--all-features
RUN --mount=type=cache,id=final,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,id=final,target=/var/lib/apt,sharing=locked \
    apt-get update && \
    # apt-get install -y libavcodec61 libavformat61 libavutil59 ca-certificates && \# Trixie
    apt-get install -y libavcodec59 libavformat59 libavutil57 ca-certificates wget && \
    test -z "${BUILD_ARGS}" || (\
        wget https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/cuda-keyring_1.1-1_all.deb && \
        dpkg -i cuda-keyring_1.1-1_all.deb && \
        apt-get update && apt-get install -y libcudnn9-cuda-12=9.1.1.17-1 libcudnn9-dev-cuda-12=9.1.1.17-1 libcudnn9-static-cuda-12=9.1.1.17-1 \
        && apt-get install -y cudnn9-cuda-12-4 libcufft-12-6 libcublas-12-6 cuda-cudart-12-6) && \
    rm -rf /var/lib/{dpkg,cache,log}/
COPY --from=builder --chown=1000:1000 /build/target/release/stemgen /usr/bin/stemgen
COPY --from=builder --chown=1000:1000 /build/target/release/libonnxruntime_providers*.so /usr/lib
ENTRYPOINT [ "/usr/bin/stemgen" ]
