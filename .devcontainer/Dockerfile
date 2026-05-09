FROM mcr.microsoft.com/devcontainers/rust:latest
RUN sudo apt-get update && \
    sudo apt-get install -y cmake libutfcpp-dev unzip libavcodec-dev libavformat-dev libavutil-dev pkg-config clang yasm && \
    wget -O taglib.zip https://github.com/taglib/taglib/archive/refs/tags/v2.2.1.zip.zip && \
    unzip taglib.zip && \
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF taglib-2.2.1 && \
    sudo make -j && \
    sudo make install && \
    wget https://github.com/wader/fq/releases/download/v0.15.1/fq_0.15.1_linux_amd64.tar.gz && \
    tar xvf fq_0.15.1_linux_amd64.tar.gz  && \
    sudo install -m 755 fq  /usr/local/bin/fq
