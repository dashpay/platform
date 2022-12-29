// Import metadata directly to do not import Node.JS server logic in browsers
const { Metadata } = require('@grpc/grpc-js/build/src/metadata');

// TODO: revisit - this behaviour potentially dangerous for usage in web-clients
// because they supposed to operate with grpc-web Metadata
/**
 * Converts any JavaScript object to grpc metadata
 *
 * @param {Object} obj
 *
 * @return {module:grpc.Metadata}
 */
function convertObjectToMetadata(obj) {
  const metadata = new Metadata();

  Object.keys(obj).forEach((key) => {
    metadata.set(key, obj[key]);
  });

  return metadata;
}

module.exports = convertObjectToMetadata;
