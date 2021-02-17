const BlockchainListener = require('../BlockchainListener');

/**
 * @typedef {waitForTransactionCommitment}
 * @param {BlockchainListener} blockchainListener
 * @param {string} hashString
 * @return {{
 *    promise: Promise<void>,
 *    detach: Function
 * }}
 */
function waitForTransactionCommitment(blockchainListener, hashString) {
  const txInBlockTopic = BlockchainListener
    .getTransactionAddedToTheBlockEventName(hashString.toLowerCase());

  let txInBlockHandler;
  let newBlockHandler;

  // Note that this will resolve only after two blocks. That is because the first block will
  // flip the 'seenTransaction' toggle to true, and transaction will become provable
  // only on the next block after the block it was included into
  const promise = new Promise(((resolve) => {
    let seenTransaction = false;

    txInBlockHandler = () => {
      seenTransaction = true;
    };

    newBlockHandler = () => {
      if (!seenTransaction) {
        return;
      }

      blockchainListener.off(BlockchainListener.EVENTS.NEW_BLOCK, newBlockHandler);

      resolve();
    };

    blockchainListener.once(txInBlockTopic, txInBlockHandler);
    blockchainListener.on(BlockchainListener.EVENTS.NEW_BLOCK, newBlockHandler);
  }));

  const detach = () => {
    blockchainListener.off(txInBlockTopic, txInBlockHandler);
    blockchainListener.off(BlockchainListener.EVENTS.NEW_BLOCK, newBlockHandler);
  };

  return {
    promise,
    detach,
  };
}

module.exports = waitForTransactionCommitment;
