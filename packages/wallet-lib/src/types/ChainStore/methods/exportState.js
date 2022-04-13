function exportState() {
  const { network, state } = this;
  const {
    fees,
    blockHeight,
    blockHeaders,
    transactions,
    instantLocks,
    addresses,
  } = state;

  const serializedState = {
    network,
    state: {
      fees,
      blockHeight,
      blockHeaders: {},
      transactions: {},
      instantLocks: {},
      addresses: {},
    },
  };

  [...blockHeaders.entries()].forEach(([blockHeaderHash, blockHeader]) => {
    serializedState.state.blockHeaders[blockHeaderHash] = blockHeader.toString();
  });

  [...transactions.entries()].forEach(([transactionHash, { transaction, metadata }]) => {
    serializedState.state.transactions[transactionHash] = {
      transaction: transaction.toString(),
      metadata,
    };
  });

  [...instantLocks.entries()].forEach(([transactionHash, instantLock]) => {
    serializedState.state.instantLocks[transactionHash] = instantLock.toString();
  });

  [...addresses.entries()].forEach(([address, addressObj]) => {
    serializedState.state.addresses[address] = addressObj;
  });

  return serializedState;
}

module.exports = exportState;
