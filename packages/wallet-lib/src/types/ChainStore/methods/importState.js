const {
  BlockHeader,
  Transaction,
  InstantLock,
} = require('@dashevo/dashcore-lib');

function importState(state) {
  const {
    network,
    state: {
      fees,
      blockHeight,
      blockHeaders,
      transactions,
      instantLocks,
      addresses,
    },
  } = state;

  this.network = network;

  this.state.fees = fees;
  this.state.blockHeight = blockHeight;

  // We actually do not import address state, but import the address itself
  // Which will force processing tx for each added address, therefore we might want to do
  // import address as the first process done
  Object.values(addresses).forEach(({ address }) => {
    this.importAddress(address);
  });

  Object.values(blockHeaders).forEach((serializedBlockHeader) => {
    this.importBlockHeader(new BlockHeader(Buffer.from(serializedBlockHeader, 'hex')));
  });
  Object.values(transactions).forEach(({ transaction: serializedTransaction, metadata }) => {
    this.importTransaction(new Transaction(Buffer.from(serializedTransaction, 'hex')), metadata);
  });
  Object.values(instantLocks).forEach((serializedInstantLock) => {
    this.importInstantLock(new InstantLock(Buffer.from(serializedInstantLock, 'hex')));
  });
}
module.exports = importState;
