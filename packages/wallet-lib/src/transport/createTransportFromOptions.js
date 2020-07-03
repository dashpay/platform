const DAPIClient = require('@dashevo/dapi-client');

const _ = require('lodash');

const DAPIClientTransport = require('./DAPIClientTransport/DAPIClientTransport');

/**
 *
 * @param {DAPIClientOptions|Transport} options
 * @returns {Transport}
 */
function createTransportFromOptions(options) {
  if (!_.isPlainObject(options)) {
    // Return transport instance
    return options;
  }

  const client = new DAPIClient(options);

  return new DAPIClientTransport(client);
}

module.exports = createTransportFromOptions;
