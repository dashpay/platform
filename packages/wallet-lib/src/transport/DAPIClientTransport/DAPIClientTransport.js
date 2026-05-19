import AbstractTransport from '../AbstractTransport.js';

/**
 * @implements {Transport}
 */
class DAPIClientTransport extends AbstractTransport {
  constructor(client) {
    super();

    this.client = client;
  }
}

import _DAPIClientTransport_disconnect from './methods/disconnect.js';
DAPIClientTransport.prototype.disconnect = _DAPIClientTransport_disconnect;
import _DAPIClientTransport_getBestBlock from './methods/getBestBlock.js';
DAPIClientTransport.prototype.getBestBlock = _DAPIClientTransport_getBestBlock;
import _DAPIClientTransport_getBestBlockHeader from './methods/getBestBlockHeader.js';
DAPIClientTransport.prototype.getBestBlockHeader = _DAPIClientTransport_getBestBlockHeader;
import _DAPIClientTransport_getBestBlockHash from './methods/getBestBlockHash.js';
DAPIClientTransport.prototype.getBestBlockHash = _DAPIClientTransport_getBestBlockHash;
import _DAPIClientTransport_getBestBlockHeight from './methods/getBestBlockHeight.js';
DAPIClientTransport.prototype.getBestBlockHeight = _DAPIClientTransport_getBestBlockHeight;
import _DAPIClientTransport_getBlockByHash from './methods/getBlockByHash.js';
DAPIClientTransport.prototype.getBlockByHash = _DAPIClientTransport_getBlockByHash;
import _DAPIClientTransport_getBlockByHeight from './methods/getBlockByHeight.js';
DAPIClientTransport.prototype.getBlockByHeight = _DAPIClientTransport_getBlockByHeight;
import _DAPIClientTransport_getBlockHeaderByHash from './methods/getBlockHeaderByHash.js';
DAPIClientTransport.prototype.getBlockHeaderByHash = _DAPIClientTransport_getBlockHeaderByHash;
import _DAPIClientTransport_getBlockHeaderByHeight from './methods/getBlockHeaderByHeight.js';
DAPIClientTransport.prototype.getBlockHeaderByHeight = _DAPIClientTransport_getBlockHeaderByHeight;
import _DAPIClientTransport_getBlockchainStatus from './methods/getBlockchainStatus.js';
DAPIClientTransport.prototype.getBlockchainStatus = _DAPIClientTransport_getBlockchainStatus;
import _DAPIClientTransport_getTransaction from './methods/getTransaction.js';
DAPIClientTransport.prototype.getTransaction = _DAPIClientTransport_getTransaction;
import _DAPIClientTransport_sendTransaction from './methods/sendTransaction.js';
DAPIClientTransport.prototype.sendTransaction = _DAPIClientTransport_sendTransaction;
import _DAPIClientTransport_getIdentityByPublicKeyHash from './methods/getIdentityByPublicKeyHash.js';
DAPIClientTransport.prototype.getIdentityByPublicKeyHash = _DAPIClientTransport_getIdentityByPublicKeyHash;
import _DAPIClientTransport_subscribeToTransactionsWithProofs from './methods/subscribeToTransactionsWithProofs.js';
DAPIClientTransport.prototype.subscribeToTransactionsWithProofs = _DAPIClientTransport_subscribeToTransactionsWithProofs;

export default DAPIClientTransport;