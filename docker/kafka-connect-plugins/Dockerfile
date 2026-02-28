# Kafka Connect image with Confluent S3 Sink Connector
#
# Base: quay.io/strimzi/kafka:0.50.1-kafka-4.1.1
# Added: Confluent S3 Sink Connector 12.1.0
#
# Plugin discovery: Kafka Connect scans /opt/kafka/plugins/ for connector JARs.
# Each connector lives in its own subdirectory (the plugin.path isolation requirement).
#
# Confluent S3 Sink 12.1.0 release:
#   https://www.confluent.io/hub/confluentinc/kafka-connect-s3
# CDN download format:
#   https://d1i4a15mxbxib1.cloudfront.net/api/plugins/confluentinc/kafka-connect-s3/versions/<ver>/confluentinc-kafka-connect-s3-<ver>.zip
#
# Built by GitHub Actions (.github/workflows/build-kafka-connect-plugins.yml)
# and pushed to ghcr.io/dolr-ai/kafka-connect-plugins:latest
FROM quay.io/strimzi/kafka:0.50.1-kafka-4.1.1

# Switch to root for package installation
USER root

ARG CONNECTOR_VERSION=12.1.0

RUN set -eux; \
    mkdir -p /opt/kafka/plugins/kafka-connect-s3; \
    # Download Confluent S3 Sink Connector ZIP from Confluent Hub CDN
    curl --fail --silent --show-error --location \
         "https://d1i4a15mxbxib1.cloudfront.net/api/plugins/confluentinc/kafka-connect-s3/versions/${CONNECTOR_VERSION}/confluentinc-kafka-connect-s3-${CONNECTOR_VERSION}.zip" \
         -o /tmp/kafka-connect-s3.zip; \
    # Unzip and copy all JARs into the plugin directory
    unzip -q /tmp/kafka-connect-s3.zip -d /tmp/kafka-connect-s3-unzip; \
    cp /tmp/kafka-connect-s3-unzip/confluentinc-kafka-connect-s3-${CONNECTOR_VERSION}/lib/*.jar \
       /opt/kafka/plugins/kafka-connect-s3/; \
    # Tidy up
    rm -rf /tmp/kafka-connect-s3.zip /tmp/kafka-connect-s3-unzip

# Restore the non-root user from the Strimzi base image
USER 1001
