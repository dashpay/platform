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

DAPIClientTransport.prototype.getBestBlock = require('./methods/getBestBlock');
DAPIClientTransport.prototype.getBestBlockHeader = require('./methods/getBestBlockHeader');
DAPIClientTransport.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
DAPIClientTransport.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
DAPIClientTransport.prototype.getBlockByHash = require('./methods/getBlockByHash');
DAPIClientTransport.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
DAPIClientTransport.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
DAPIClientTransport.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
DAPIClientTransport.prototype.getStatus = require('./methods/getStatus');
DAPIClientTransport.prototype.getTransaction = require('./methods/getTransaction');
DAPIClientTransport.prototype.sendTransaction = require('./methods/sendTransaction');
DAPIClientTransport.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
DAPIClientTransport.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');
DAPIClientTransport.prototype.getIdentityIdByFirstPublicKey = require('./methods/getIdentityIdByFirstPublicKey');
DAPIClientTransport.prototype.subscribeToTransactionsWithProofs = require('./methods/subscribeToTransactionsWithProofs');

module.exports = DAPIClientTransport;
