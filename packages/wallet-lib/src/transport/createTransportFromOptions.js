const DAPIClient = require('@dashevo/dapi-client');

const _ = require('lodash');

const DAPIClientTransport = require('./DAPIClientTransport/DAPIClientTransport');

/**
 *
 * @param {DAPIClientOptions|Transport|DAPIClientTransport} options
 * @returns {Transport|DAPIClientTransport}
 */
function createTransportFromOptions(options) {
  let transport;
  if (!_.isPlainObject(options)) {
    // Return transport instance
    transport = options;
  } else {
    const client = new DAPIClient(options);

    // TODO: handle errors from DAPIClient there

    transport = new DAPIClientTransport(client);
  }

  transport.client.blockHeadersProvider
    .on(DAPIClient.BlockHeadersProvider.EVENTS.BATCH_OF_HEADERS_VERIFIED, (headers) => {
      console.log(`[BlockHeadersProvider] verified ${headers.length} headers`);
    });

  transport.client.blockHeadersProvider
    .on(DAPIClient.BlockHeadersProvider.EVENTS.HISTORICAL_SYNC_FINISHED, () => {
      console.log('[BlockHeadersProvider] Historical sync finished');
    });

  return transport;
}

module.exports = createTransportFromOptions;
