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

  it('should behave like an event bus', () => {
    expect(readerMediator).to.be.an.instanceOf(Emittery);
  });

  it('should return initial block height', () => {
    expect(readerMediator.getInitialBlockHeight()).to.equal(initialBlockHeight);
  });

  it('should return reader state', () => {
    expect(readerMediator.getState()).to.equal(stateMock);
  });

  it('should reset the state and emit an event');
});
