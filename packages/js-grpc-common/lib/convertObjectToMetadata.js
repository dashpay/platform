const { Metadata } = require('@grpc/grpc-js');

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
    const value = JSON.stringify(obj[key]);
    metadata.set(key, value);
  });

  return metadata;
}

module.exports = convertObjectToMetadata;
