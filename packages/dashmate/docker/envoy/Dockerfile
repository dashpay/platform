FROM envoyproxy/envoy:v1.22-latest

RUN apt update && \
    apt -qq -y install python && \
    rm -rf /var/lib/apt/lists/*

COPY hot-restarter.py /etc/envoy/hot-restarter.py
COPY start_envoy.sh /etc/envoy/start_envoy.sh

ENTRYPOINT ["/etc/envoy/hot-restarter.py", "/etc/envoy/start_envoy.sh"]
