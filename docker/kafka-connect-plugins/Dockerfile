# Kafka Connect image with Confluent S3 Sink Connector
#
# Base: quay.io/strimzi/kafka:0.50.1-kafka-4.1.1
# Added: Confluent S3 Sink Connector 12.1.0
#
# Plugin discovery: Kafka Connect scans /opt/kafka/plugins/ for connector JARs.
# Each connector lives in its own subdirectory (the plugin.path isolation requirement).
#
# Download URL sourced from: https://api.hub.confluent.io/api/plugins/confluentinc/kafka-connect-s3/versions/12.1.0
#
# Built by GitHub Actions (.github/workflows/build-kafka-connect-plugins.yml)
# and pushed to ghcr.io/dolr-ai/kafka-connect-plugins:latest
FROM quay.io/strimzi/kafka:1.0.0-kafka-4.2.0

ARG CONNECTOR_VERSION=12.1.0

# Switch to root to install the connector
USER root

RUN set -eux; \
    mkdir -p /opt/kafka/plugins/kafka-connect-s3; \
    curl --fail --silent --show-error --location \
         "https://hub-downloads.confluent.io/api/plugins/confluentinc/kafka-connect-s3/versions/${CONNECTOR_VERSION}/confluentinc-kafka-connect-s3-${CONNECTOR_VERSION}.zip" \
         -o /tmp/kafka-connect-s3.zip; \
    unzip -q /tmp/kafka-connect-s3.zip -d /tmp/kafka-connect-s3-unzip; \
    cp /tmp/kafka-connect-s3-unzip/confluentinc-kafka-connect-s3-${CONNECTOR_VERSION}/lib/*.jar \
       /opt/kafka/plugins/kafka-connect-s3/; \
    rm -rf /tmp/kafka-connect-s3.zip /tmp/kafka-connect-s3-unzip

# Restore the non-root user from the Strimzi base image
USER 1001
