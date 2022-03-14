const {
  BlockHeader,
  Transaction,
  InstantLock,
} = require('@dashevo/dashcore-lib');

function importState(state) {
  const {
    blockHeaders,
    transactions,
    instantLocks,
    txMetadata,
  } = state;

  // TODO: check whether we need it
  // We actually do not import address state, but import the address itself
  // Which will force processing tx for each added address, therefore we might want to do
  // import address as the first process done
  // Object.values(addresses).forEach(({ address }) => {
  //   this.importAddress(address);
  // });

  Object.values(blockHeaders).forEach((serializedBlockHeader) => {
    this.importBlockHeader(new BlockHeader(Buffer.from(serializedBlockHeader, 'hex')));
  });

  Object.keys(transactions).forEach((hash) => {
    const tx = new Transaction(Buffer.from(transactions[hash], 'hex'));
    const metadata = txMetadata[hash];
    this.importTransaction(tx, metadata);
  });

  Object.values(instantLocks).forEach((serializedInstantLock) => {
    this.importInstantLock(new InstantLock(Buffer.from(serializedInstantLock, 'hex')));
  });
}
module.exports = importState;
