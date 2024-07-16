const AbstractTransport = require('../AbstractTransport');

/**
 * @implements {Transport}
 */
class DAPIClientTransport extends AbstractTransport {
  constructor(client) {
    super();

    this.client = client;
  }
}

DAPIClientTransport.prototype.disconnect = require('./methods/disconnect');
DAPIClientTransport.prototype.getBestBlock = require('./methods/getBestBlock');
DAPIClientTransport.prototype.getBestBlockHeader = require('./methods/getBestBlockHeader');
DAPIClientTransport.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
DAPIClientTransport.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
DAPIClientTransport.prototype.getBlockByHash = require('./methods/getBlockByHash');
DAPIClientTransport.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
DAPIClientTransport.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
DAPIClientTransport.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
DAPIClientTransport.prototype.getBlockchainStatus = require('./methods/getBlockchainStatus');
DAPIClientTransport.prototype.getTransaction = require('./methods/getTransaction');
DAPIClientTransport.prototype.sendTransaction = require('./methods/sendTransaction');
DAPIClientTransport.prototype.getIdentityByPublicKeyHash = require('./methods/getIdentityByPublicKeyHash');
DAPIClientTransport.prototype.subscribeToTransactionsWithProofs = require('./methods/subscribeToTransactionsWithProofs');

module.exports = DAPIClientTransport;
