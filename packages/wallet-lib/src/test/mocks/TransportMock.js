const EventEmitter = require('events');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');
const getBlockchainStatus = require('../../transport/FixtureTransport/methods/getBlockchainStatus');

class TransportMock extends EventEmitter {
  constructor(sinon, transactionStreamMock) {
    super();
    this.sinon = sinon;

    this.height = 42;

    this.getBestBlockHeight = sinon.stub().returns(42);
    this.subscribeToTransactionsWithProofs = sinon.stub().returns(transactionStreamMock);
    this.getBlockHeaderByHeight = sinon.stub()
      .returns({
        hash: '000000059885815cfc06ba74b814200d29658394dbe5d1e93948a8587947747b',
        version: 536870912,
        prevHash: '000000c520efd2047f0b6f0c1c75e0382f8a9b7d76bb140bde3ada10c62e8b0d',
        merkleRoot: 'ef292bfb7965402e57dfeb4ee8bad0055c216c4c5a4e549a0ac17a393ae8617b',
        time: 1638950949,
        bits: 503385436,
        nonce: 351770,
      });
    this.subscribeToBlocks = sinon.stub();
    this.getIdentityByPublicKeyHash = sinon.stub()
      .rejects(new NotFoundError('Identity not found', {}, null));
    this.sendTransaction = sinon.stub();
    this.getTransaction = sinon.stub();
    this.getBlockHeaderByHash = sinon.stub();
    this.getBlockchainStatus = sinon.stub().resolves(getBlockchainStatus.call(this));

    const provider = new EventEmitter();
    provider.stop = sinon.stub().callsFake(() => {
      provider.emit('STOPPED');
    });
    provider.initializeChainWith = sinon.spy();
    provider.readHistorical = sinon.spy();
    provider.startContinuousSync = sinon.spy();
    provider.spvChain = {
      getLongestChain() {
        return [];
      },
      orphanChunks: [],
      startBlockHeight: 1,
    };

    this.client = {
      blockHeadersProvider: provider,
      core: {
        subscribeToTransactionsWithProofs: sinon.stub().returns(transactionStreamMock),
      },
    };
  }
}

module.exports = TransportMock;
