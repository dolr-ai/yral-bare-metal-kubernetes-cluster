# Kafka Connect image with Confluent S3 Sink Connector
#
# Base: quay.io/strimzi/kafka:0.50.1-kafka-4.1.1
# Added: Confluent S3 Sink Connector 12.1.0
#
# Plugin discovery: Kafka Connect scans /opt/kafka/plugins/ for connector JARs.
# Each connector lives in its own subdirectory (the plugin.path isolation requirement).
#
# The connector ZIP is downloaded by the GitHub Actions workflow on the runner
# (not inside Docker BuildKit) and placed at plugins/kafka-connect-s3.zip in
# the build context before `docker build` runs. This avoids DNS resolution
# issues inside the BuildKit network and keeps the image build fully hermetic.
#
# Built by GitHub Actions (.github/workflows/build-kafka-connect-plugins.yml)
# and pushed to ghcr.io/dolr-ai/kafka-connect-plugins:latest
FROM quay.io/strimzi/kafka:0.50.1-kafka-4.1.1

ARG CONNECTOR_VERSION=12.1.0

# Switch to root to install the connector
USER root

# The ZIP is downloaded by the GHA workflow and passed in via the build context
COPY plugins/kafka-connect-s3.zip /tmp/kafka-connect-s3.zip

RUN set -eux; \
    mkdir -p /opt/kafka/plugins/kafka-connect-s3; \
    unzip -q /tmp/kafka-connect-s3.zip -d /tmp/kafka-connect-s3-unzip; \
    cp /tmp/kafka-connect-s3-unzip/confluentinc-kafka-connect-s3-${CONNECTOR_VERSION}/lib/*.jar \
       /opt/kafka/plugins/kafka-connect-s3/; \
    rm -rf /tmp/kafka-connect-s3.zip /tmp/kafka-connect-s3-unzip

# Restore the non-root user from the Strimzi base image
USER 1001
