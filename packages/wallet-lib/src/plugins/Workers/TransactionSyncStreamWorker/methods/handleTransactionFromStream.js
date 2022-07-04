const logger = require('../../../../logger');
const EVENTS = require('../../../../EVENTS');
const ChainSyncMediator = require('../../../../types/Wallet/ChainSyncMediator');

// TODO - remove
async function handleTransactionFromStream(transaction) {
  const self = this;
  // As we require height information, we fetch transaction using client.
  // eslint-disable-next-line no-restricted-syntax
  // eslint-disable-next-line no-underscore-dangle
  const transactionHash = transaction.hash;

  this.pendingRequest[transactionHash] = { isProcessing: true, type: 'transaction' };

  let getTransactionResponse;

  // We use BlockHeadersStream to find out TX metadata in the state of historical sync
  if (this.chainSyncMediator.state === ChainSyncMediator.STATES.HISTORICAL_SYNC) {
    const blockHash = this.chainSyncMediator.transactionsBlockHashes[transactionHash];
    const blockHeight = this.chainSyncMediator.blockHeights[blockHash];

    getTransactionResponse = {
      blockHash,
      height: blockHeight,
      isChainLocked: false,
      isInstantLocked: false,
      transaction,
    };
  } else {
    // We use DAPI getTransaction during the historical sync because it seems that
    // there's a bug with the delivery of merkle blocks
    // (Or apparently next processChunk stucks until this one gets processed?)
    getTransactionResponse = await this.transport.getTransaction(transactionHash);
  }
  // console.log('[handleTransactionFromStream]',
  // transactionHash, getTransactionResponse.blockHash, getTransactionResponse.height);

  // if (getTransactionResponse.blockHash && !this.) {
  //   This can happen due to propagation when one node inform us about a transaction,
  //   but the node we ask the transaction to is not aware of it.
  // logger.silly(`TransactionSyncStreamWorker - Transaction ${transactionHash} was not found`);
  //
  // }

  if (!getTransactionResponse.height) {
    // Retry quickly as we are in the process of historical sync and block header should arrive soon
    if (this.chainSyncMediator.state === ChainSyncMediator.STATES.HISTORICAL_SYNC) {
      // console.log('[handleTransactionFromStream] Quick retry', transactionHash);
      return new Promise((resolve) => {
        setTimeout(() => {
          resolve(self.handleTransactionFromStream(transaction));
        }, 1000);
      });
    }
    // console.log(`[handleTransactionFromStream] tx ${transactionHash} wait for block`);
    // at this point, transaction is not yet mined, therefore we gonna retry on next block to
    // fetch this tx and subsequently its blockhash for blockheader fetching.
    logger.silly(`TransactionSyncStreamWorker - Unconfirmed transaction ${transactionHash}: delayed.`);
    const existingListener = this.delayedRequests[transactionHash]
      && this.delayedRequests[transactionHash].blockHeightChangeListener;

    if (existingListener) {
      self.parentEvents.removeListener(EVENTS.BLOCKHEIGHT_CHANGED, existingListener);
    }

    this.delayedRequests[transactionHash] = { isDelayed: true, type: 'transaction', blockHeightChangeListener: null };

    return new Promise((resolve) => {
      const blockHeightChangeListener = (data) => {
        // console.log('[handleTransactionFromStream] Height changed!', data);
        resolve(self.handleTransactionFromStream(transaction));
      };

      this.delayedRequests[transactionHash].blockHeightChangeListener = blockHeightChangeListener;

      self.parentEvents.once(
        EVENTS.BLOCKHEIGHT_CHANGED,
        blockHeightChangeListener,
      );
    });
  }

  const executor = async () => {
    if (self.delayedRequests[transactionHash]) {
      logger.silly(`TransactionSyncStreamWorker - Processing previously delayed transaction ${transactionHash} from stream`);
      const { blockHeightChangeListener } = self.delayedRequests[transactionHash];
      if (blockHeightChangeListener) {
        self.parentEvents.removeListener(EVENTS.BLOCKHEIGHT_CHANGED, blockHeightChangeListener);
      }
      delete self.delayedRequests[transactionHash];
    } else {
      logger.silly(`TransactionSyncStreamWorker - Processing transaction ${transactionHash} from stream`);
    }

    this.pendingRequest[getTransactionResponse.blockHash.toString('hex')] = { isProcessing: true, type: 'blockheader' };
    // TODO: pay attention. moved to BlockHeadersSyncWorker
    // eslint-disable-next-line no-await-in-loop
    // const getBlockHeaderResponse = await this
    //   .transport
    //   .getBlockHeaderByHash(getTransactionResponse.blockHash);
    // eslint-disable-next-line no-await-in-loop
    // await this.importBlockHeader(getBlockHeaderResponse);
    delete this.pendingRequest[getTransactionResponse.blockHash.toString('hex')];
  };

  await executor();

  const metadata = {
    blockHash: getTransactionResponse.blockHash,
    height: getTransactionResponse.height,
    instantLocked: getTransactionResponse.isInstantLocked,
    chainLocked: getTransactionResponse.isChainLocked,
  };

  delete this.pendingRequest[transactionHash];

  return {
    transaction,
    transactionHash,
    metadata,
    transactionResponse: getTransactionResponse,
  };
}

module.exports = handleTransactionFromStream;
