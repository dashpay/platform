const { Metadata: MetadataGrpcJS } = require('@grpc/grpc-js/build/src/metadata');
const { grpc: { Metadata: MetadataGrpcWeb } } = require('@improbable-eng/grpc-web');

/**
 *
 * @param {MetadataGrpcJS|MetadataGrpcWeb} metadata
 * @return {{}|{[p: string]: string}}
 */
const parseMetadata = (metadata) => {
  if (metadata instanceof MetadataGrpcJS) {
    return metadata.getMap();
  } if (metadata instanceof MetadataGrpcWeb) {
    const parsedMetadata = {};
    metadata.forEach((key, values) => {
      // Mapping each key to the first value associated with it.
      // This corresponds to the getMap() behaviour of the grpc-js Metadata class.
      const [value] = values;
      parsedMetadata[key] = value;
    });

    return parsedMetadata;
  }

  return metadata;
};

module.exports = parseMetadata;
