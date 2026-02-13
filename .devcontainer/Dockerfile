FROM mcr.microsoft.com/devcontainers/rust:latest
RUN sudo apt-get update && \
    sudo apt-get install -y cmake libutfcpp-dev unzip libasound2-dev libavcodec-dev libavformat-dev libavutil-dev pkg-config clang yasm libasound2-dev protobuf-compiler libxkbcommon-x11-0 zenity && \
    wget -O taglib.zip https://github.com/acolombier/taglib/archive/refs/heads/feat/mp4-ni-stem.zip && \
    unzip taglib.zip && \
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF taglib-feat-mp4-ni-stem && \
    sudo make -j && \
    sudo make install && \
    wget https://github.com/wader/fq/releases/download/v0.15.1/fq_0.15.1_linux_amd64.tar.gz && \
    tar xvf fq_0.15.1_linux_amd64.tar.gz  && \
    sudo install -m 755 fq  /usr/local/bin/fq
