const broadcastTransactionFactory = require('./broadcastTransactionFactory');
const generateToAddressFactory = require('./generateToAddressFactory');
const getBestBlockHashFactory = require('./getBestBlockHashFactory');
const getBlockByHashFactory = require('./getBlockByHashFactory');
const getBlockByHeightFactory = require('./getBlockByHeightFactory');
const getBlockHashFactory = require('./getBlockHashFactory');
const getMnListDiffFactory = require('./getMnListDiffFactory');
const getCoreChainStatusFactory = require('./getCoreChainStatusFactory');
const getMasternodeStatusFactory = require('./getMasternodeStatusFactory');
const getTransactionFactory = require('./getTransaction/getTransactionFactory');
const subscribeToTransactionsWithProofsFactory = require('./subscribeToTransactionsWithProofsFactory');
const subscribeToBlockHeadersWithChainLocksFactory = require('./subscribeToBlockHeadersWithChainLocksFactory');

class CoreMethodsFacade {
  /**
   * @param {JsonRpcTransport} jsonRpcTransport
   * @param {GrpcTransport} grpcTransport
   */
  constructor(jsonRpcTransport, grpcTransport) {
    this.broadcastTransaction = broadcastTransactionFactory(grpcTransport);
    this.generateToAddress = generateToAddressFactory(jsonRpcTransport);
    this.getBestBlockHash = getBestBlockHashFactory(jsonRpcTransport);
    this.getBlockByHash = getBlockByHashFactory(grpcTransport);
    this.getBlockByHeight = getBlockByHeightFactory(grpcTransport);
    this.getBlockHash = getBlockHashFactory(jsonRpcTransport);
    this.getMnListDiff = getMnListDiffFactory(jsonRpcTransport);
    this.getCoreChainStatus = getCoreChainStatusFactory(grpcTransport);
    this.getMasternodeStatus = getMasternodeStatusFactory(grpcTransport);
    this.getTransaction = getTransactionFactory(grpcTransport);
    this.subscribeToTransactionsWithProofs = subscribeToTransactionsWithProofsFactory(
      grpcTransport,
    );
    this.subscribeToBlockHeadersWithChainLocks = subscribeToBlockHeadersWithChainLocksFactory(
      grpcTransport,
    );
  }
}

module.exports = CoreMethodsFacade;
