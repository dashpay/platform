class TransportMock {
  constructor(sinonSandbox, transactionStreamMock) {
    this.sinonSandbox = sinonSandbox;

    this.getBestBlockHeight = sinonSandbox.stub().returns(42);
    this.subscribeToTransactionsWithProofs = sinonSandbox.stub().returns(transactionStreamMock);
    this.getBlockHeaderByHeight = sinonSandbox.stub().returns({ hash: 123 });
    this.on = sinonSandbox.stub();
    this.subscribeToBlocks = sinonSandbox.stub();
    this.getIdentityIdsByPublicKeyHash = sinonSandbox.stub().returns([null]);
    this.sendTransaction = sinonSandbox.stub();
  }
}

module.exports = TransportMock;
