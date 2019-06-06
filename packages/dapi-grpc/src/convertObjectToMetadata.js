const { Metadata } = require('grpc');

/**
 * Converts any JavaScript object to grpc metadata
 * @param {Object} obj
 * @returns {module:grpc.Metadata}
 */
function convertObjectToMetadata(obj) {
  const metadata = new Metadata();

  Object.keys(obj).forEach((key) => {
    metadata.set(key, obj[key]);
  });

  return metadata;
}

module.exports = convertObjectToMetadata;
