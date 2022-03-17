const {
  BlockHeader,
  Transaction,
  InstantLock,
} = require('@dashevo/dashcore-lib');

function importState(state) {
  const {
    blockHeaders,
    transactions,
    txMetadata,
  } = state;

  Object.values(blockHeaders).forEach((serializedBlockHeader) => {
    this.importBlockHeader(new BlockHeader(Buffer.from(serializedBlockHeader, 'hex')));
  });

  Object.keys(transactions).forEach((hash) => {
    const tx = new Transaction(Buffer.from(transactions[hash], 'hex'));
    const metadata = txMetadata[hash];
    this.importTransaction(tx, metadata);
  });
}
module.exports = importState;
