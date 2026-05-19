import broadcastTransactionFactory from './broadcastTransactionFactory.js';
import getBestBlockHashFactory from './getBestBlockHashFactory.js';
import getBestBlockHeightFactory from './getBestBlockHeightFactory.js';
import getBlockByHashFactory from './getBlockByHashFactory.js';
import getBlockByHeightFactory from './getBlockByHeightFactory.js';
import getBlockHashFactory from './getBlockHashFactory.js';
import getBlockchainStatusFactory from './getBlockchainStatusFactory.js';
import getMasternodeStatusFactory from './getMasternodeStatusFactory.js';
import getTransactionFactory from './getTransaction/getTransactionFactory.js';
import subscribeToTransactionsWithProofsFactory from './subscribeToTransactionsWithProofsFactory.js';
import subscribeToBlockHeadersWithChainLocksFactory from './subscribeToBlockHeadersWithChainLocksFactory.js';
import subscribeToToMasternodeListFactory from './subscribeToMasternodeListFactory.js';

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

export default CoreMethodsFacade;
