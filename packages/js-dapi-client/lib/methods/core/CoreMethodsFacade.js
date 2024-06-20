const broadcastTransactionFactory = require('./broadcastTransactionFactory');
const getBestBlockHashFactory = require('./getBestBlockHashFactory');
const getBestBlockHeightFactory = require('./getBestBlockHeightFactory');
const getBlockByHashFactory = require('./getBlockByHashFactory');
const getBlockByHeightFactory = require('./getBlockByHeightFactory');
const getBlockHashFactory = require('./getBlockHashFactory');
const getBlockchainStatusFactory = require('./getBlockchainStatusFactory');
const getMasternodeStatusFactory = require('./getMasternodeStatusFactory');
const getTransactionFactory = require('./getTransaction/getTransactionFactory');
const subscribeToTransactionsWithProofsFactory = require('./subscribeToTransactionsWithProofsFactory');
const subscribeToBlockHeadersWithChainLocksFactory = require('./subscribeToBlockHeadersWithChainLocksFactory');
const subscribeToToMasternodeListFactory = require('./subscribeToMasternodeListFactory');

class CoreMethodsFacade {
  /**
   * @param {JsonRpcTransport} jsonRpcTransport
   * @param {GrpcTransport} grpcTransport
   */
  constructor(jsonRpcTransport, grpcTransport) {
    this.broadcastTransaction = broadcastTransactionFactory(grpcTransport);
    this.getBestBlockHash = getBestBlockHashFactory(jsonRpcTransport);
    this.getBestBlockHeight = getBestBlockHeightFactory(grpcTransport);
    this.getBlockByHash = getBlockByHashFactory(grpcTransport);
    this.getBlockByHeight = getBlockByHeightFactory(grpcTransport);
    this.getBlockHash = getBlockHashFactory(jsonRpcTransport);
    this.getBlockchainStatus = getBlockchainStatusFactory(grpcTransport);
    this.getMasternodeStatus = getMasternodeStatusFactory(grpcTransport);
    this.getTransaction = getTransactionFactory(grpcTransport);
    this.subscribeToTransactionsWithProofs = subscribeToTransactionsWithProofsFactory(
      grpcTransport,
    );
    this.subscribeToBlockHeadersWithChainLocks = subscribeToBlockHeadersWithChainLocksFactory(
      grpcTransport,
    );
    this.subscribeToMasternodeList = subscribeToToMasternodeListFactory(
      grpcTransport,
    );
  }
}

module.exports = CoreMethodsFacade;
