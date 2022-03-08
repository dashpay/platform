const logger = require('../../../../logger');
const EVENTS = require('../../../../EVENTS');

async function handleTransactionFromStream(transaction) {
  const self = this;
  // As we require height information, we fetch transaction using client.
  // eslint-disable-next-line no-restricted-syntax
  // eslint-disable-next-line no-underscore-dangle
  const transactionHash = transaction.hash;

  this.pendingRequest[transactionHash] = { isProcessing: true, type: 'transaction' };
  // eslint-disable-next-line no-await-in-loop
  const getTransactionResponse = await this.transport.getTransaction(transactionHash);

  if (!getTransactionResponse) {
    // This can happen due to propagation when one node inform us about a transaction,
    // but the node we ask the transaction to is not aware of it.
    logger.silly(`TransactionSyncStreamWorker - Transaction ${transactionHash} was not found`);
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve(self.handleTransactionFromStream(transaction));
      }, 1000);
    });
  }

  if (!getTransactionResponse.blockHash) {
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
      const blockHeightChangeListener = () => {
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
    // eslint-disable-next-line no-await-in-loop
    const getBlockHeaderResponse = await this
      .transport
      .getBlockHeaderByHash(getTransactionResponse.blockHash);
    // eslint-disable-next-line no-await-in-loop
    await this.importBlockHeader(getBlockHeaderResponse);
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
