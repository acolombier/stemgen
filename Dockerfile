FROM python:3.10 as build
WORKDIR /build
RUN wget -O taglib.tar.gz https://github.com/taglib/taglib/releases/download/v2.0.1/taglib-2.0.1.tar.gz && \
    tar xf taglib.tar.gz  && \
    apt update && apt install -y cmake libutfcpp-dev && \
    cmake -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=ON taglib-2.0.1 && \
    make -j && \
    make install && \
    pip install build
ADD . .
RUN python3 -m build --wheel && \
    find /usr/lib -name libtag.so.2.0.1 -exec cp '{}' /build/libtag.so.2.0.1 \;


FROM python:3.10
COPY --from=build /build/libtag.so.2.0.1 /usr/local/lib/libtag.so.2.0.1
COPY --from=build /build/dist/stemgen-*.whl /tmp/
COPY --from=build /usr/include/taglib/ /usr/include/taglib/
RUN ln -s /usr/local/lib/libtag.so.2.0.1 /usr/local/lib/libtag.so && \
    python -m pip install /tmp/stemgen-*.whl && \
    apt update && apt install -y ffmpeg && \
    pip install --upgrade --force torchaudio && \
    rm -rf /root/.cache /tmp/* && rm -rf /usr/include/taglib/
CMD ["/usr/local/bin/stemgen"]
