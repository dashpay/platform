FROM envoyproxy/envoy:v1.22-latest as builder

RUN apt update && \
    apt -qq -y install python

COPY packages/dashmate/docker/envoy/hot-restarter.py /etc/envoy/hot-restarter.py
COPY packages/dashmate/docker/envoy/start_envoy.sh /etc/envoy/start_envoy.sh

ENTRYPOINT ["/etc/envoy/hot-restarter.py", "/etc/envoy/start_envoy.sh"]
