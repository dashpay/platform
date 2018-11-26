const Emittery = require('emittery');

const BlockchainReaderMediator = require('../../../../lib/blockchain/reader/BlockchainReaderMediator');

describe('BlockchainReaderMediator', () => {
  let stateMock;
  let readerMediator;
  let initialBlockHeight;

  beforeEach(() => {
    stateMock = {};
    initialBlockHeight = 2;
    readerMediator = new BlockchainReaderMediator(stateMock, initialBlockHeight);
  });

  it('should be events bus', () => {
    expect(readerMediator).to.be.instanceOf(Emittery);
  });

  it('should return initial block height', () => {
    expect(readerMediator.getInitialBlockHeight()).to.be.equal(initialBlockHeight);
  });

  it('should return reader state', () => {
    expect(readerMediator.getState()).to.be.equal(stateMock);
  });

  it('should reset the state and emit an event');
});
