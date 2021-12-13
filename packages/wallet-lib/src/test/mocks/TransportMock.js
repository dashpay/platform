const getStatus = require('../../transport/FixtureTransport/methods/getStatus');

class TransportMock {
  constructor(sinonSandbox, transactionStreamMock) {
    this.sinonSandbox = sinonSandbox;

    this.getBestBlockHeight = sinonSandbox.stub().returns(42);
    this.subscribeToTransactionsWithProofs = sinonSandbox.stub().returns(transactionStreamMock);
    this.getBlockHeaderByHeight = sinonSandbox.stub()
      .returns({
        hash: '000000059885815cfc06ba74b814200d29658394dbe5d1e93948a8587947747b',
        version: 536870912,
        prevHash: '000000c520efd2047f0b6f0c1c75e0382f8a9b7d76bb140bde3ada10c62e8b0d',
        merkleRoot: 'ef292bfb7965402e57dfeb4ee8bad0055c216c4c5a4e549a0ac17a393ae8617b',
        time: 1638950949,
        bits: 503385436,
        nonce: 351770,
      });
    this.on = sinonSandbox.stub();
    this.subscribeToBlocks = sinonSandbox.stub();
    this.getIdentityIdsByPublicKeyHash = sinonSandbox.stub().returns([null]);
    this.sendTransaction = sinonSandbox.stub();
    this.getTransaction = sinonSandbox.stub();
    this.getBlockHeaderByHash = sinonSandbox.stub();
    this.getStatus = sinonSandbox.stub().resolves(getStatus.call(this));
  }
}

module.exports = TransportMock;
