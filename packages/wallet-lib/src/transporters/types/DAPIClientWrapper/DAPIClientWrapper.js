const BaseTransporter = require('../BaseTransporter/BaseTransporter');
const logger = require('../../../logger');

const defaultDAPIOpts = {
  seeds: [
    { service: 'seed-1.evonet.networks.dash.org' },
    { service: 'seed-2.evonet.networks.dash.org' },
    { service: 'seed-3.evonet.networks.dash.org' },
    { service: 'seed-4.evonet.networks.dash.org' },
    { service: 'seed-5.evonet.networks.dash.org' },
  ],
  timeout: 20000,
  retries: 5,
};

/**
 * Creates a new DAPIClientWrapper; holds a DAPIClient instance initialized with passed params
 * @param [props=defaultDAPIOpts]
 * @param {Array<Object>} [options.seeds] - If no seeds provided default seeds will be used.
 * @param {number} [options.port] - default port for connection to the DAPI
 * @param {number} [options.nativeGrpcPort] - Native GRPC port for connection to the DAPI
 * @param {number} [options.timeout] - timeout for connection to the DAPI
 * @param {number} [options.retries] - num of retries if there is no response from DAPI node
 * @constructor
 */
class DAPIClientWrapper extends BaseTransporter {
  constructor(props) {
    super({ ...props, type: 'DAPIClientWrapper' });
    try {
      // This allows to not have dapi-client shipped by default.
      // eslint-disable-next-line global-require,import/no-extraneous-dependencies
      const Client = require('@dashevo/dapi-client');
      this.client = new Client({ ...defaultDAPIOpts, ...props });
    } catch (err) {
      logger.error("The '@dashevo/dapi-client' package is missing! Please install it with 'npm install @dashevo/dapi-client --save' command.");
    }
  }
}

DAPIClientWrapper.prototype.disconnect = require('./methods/disconnect');
DAPIClientWrapper.prototype.getAddressSummary = require('./methods/getAddressSummary');
DAPIClientWrapper.prototype.getBestBlock = require('./methods/getBestBlock');
DAPIClientWrapper.prototype.getBestBlockHeader = require('./methods/getBestBlockHeader');
DAPIClientWrapper.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
DAPIClientWrapper.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
DAPIClientWrapper.prototype.getBlockHash = require('./methods/getBlockHash');
DAPIClientWrapper.prototype.getBlockByHash = require('./methods/getBlockByHash');
DAPIClientWrapper.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
DAPIClientWrapper.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
DAPIClientWrapper.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
DAPIClientWrapper.prototype.getStatus = require('./methods/getStatus');
DAPIClientWrapper.prototype.getTransaction = require('./methods/getTransaction');
DAPIClientWrapper.prototype.getUTXO = require('./methods/getUTXO');
DAPIClientWrapper.prototype.sendTransaction = require('./methods/sendTransaction');
DAPIClientWrapper.prototype.subscribeToAddressesTransactions = require('./methods/subscribeToAddressesTransactions');
DAPIClientWrapper.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
DAPIClientWrapper.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');

module.exports = DAPIClientWrapper;
