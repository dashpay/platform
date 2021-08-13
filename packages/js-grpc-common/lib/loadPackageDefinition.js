const grpc = require('grpc');
const protoLoader = require('@grpc/proto-loader');

const lodashGet = require('lodash.get');

/**
 * Load GRPC package definition
 *
 * @param {string} protoPath
 * @param {string} [namespace]
 *
 * @return {*}
 */
function loadPackageDefinition(protoPath, namespace = undefined) {
  const definition = protoLoader.loadSync(protoPath, {
    keepCase: false,
    longs: String,
    enums: String,
    bytes: Uint8Array,
    defaults: true,
  });

  const packageDefinition = grpc.loadPackageDefinition(definition);

  if (namespace) {
    return lodashGet(packageDefinition, namespace);
  }

  return packageDefinition;
}

module.exports = loadPackageDefinition;
