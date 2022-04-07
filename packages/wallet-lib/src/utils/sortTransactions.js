/**
 * @typedef TxMetadata
 * @property {number} height
 * @property {string} blockHash
 * @property {boolean} isChainLocked
 * @property {boolean} isInstantLocked
 */

/**
 * Sorts transactions by height taking into account prevTx linkage within the same height
 * @typedef sortTransactions
 * @param {{ transaction: Transaction, metadata: TxMetadata }} txsWithMetadata
 * @returns {Transaction[]}
 */
const sortTransactions = (txsWithMetadata) => {
  const transactionsByHeight = txsWithMetadata.reduce((acc, { transaction, metadata }) => {
    const { height } = metadata;

    if (!acc[height]) {
      acc[height] = [];
    }

    acc[height].push(transaction);

    return acc;
  }, {});

  return Object.keys(transactionsByHeight)
    .sort((a, b) => parseInt(a, 10) - parseInt(b, 10))
    .reduce((acc, height) => {
      transactionsByHeight[height].sort((a, b) => {
        // const prevTxHashBuffer = Buffer.alloc(32);
        let prevTxHashBuffer = null;
        b.inputs.forEach((input) => {
          if (input.prevTxId.readUInt32BE() !== 0) {
            prevTxHashBuffer = input.prevTxId;
          }
        });
        const prevTxHash = prevTxHashBuffer.toString('hex');
        if (a.hash === prevTxHash) {
          return -1;
        }
        return 0;
      });
      return acc.concat(transactionsByHeight[height]);
    }, []);
};

module.exports = sortTransactions;
