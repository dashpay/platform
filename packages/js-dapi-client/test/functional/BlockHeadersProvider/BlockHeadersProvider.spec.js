const EventEmitter = require('events');
const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');

const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersProvider - functional test', () => {
  let coreApiMock;
  let blockHeadersReaderMock;
  let blockHeadersProvider;
  let spvChainMock;

  beforeEach(function () {
    blockHeadersReaderMock = new EventEmitter();
    blockHeadersReaderMock.subscribeToNew = this.sinon.stub();
    blockHeadersReaderMock.readHistorical = this.sinon.stub();

    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
      getStatus: this.sinon.stub().resolves({
        chain: {
          blocksCount: -1,
        },
      }),
    };

    spvChainMock = {
      addHeaders: this.sinon.stub(),
    };

    blockHeadersProvider = new BlockHeadersProvider();
    blockHeadersProvider.setCoreMethods(coreApiMock);
    blockHeadersProvider.setSpvChain(spvChainMock);
    blockHeadersProvider.setBlockHeadersReader(blockHeadersReaderMock);
  });

  it('should throw error in case core methods are missing', async () => {
    blockHeadersProvider.setCoreMethods(null);
    try {
      await blockHeadersProvider.start();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
    }
  });

  it('should throw error in case BlockHeadersProvider has already been started', async () => {
    await blockHeadersProvider.start();

    try {
      await blockHeadersProvider.start();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
    }
  });

  it('should emit ERROR event in case BlockHeadersReader emits ERROR', async () => {
    await blockHeadersProvider.start();

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    const errorToThrow = new Error('test');
    blockHeadersReaderMock.emit(BlockHeadersProvider.EVENTS.ERROR, errorToThrow);

    expect(emittedError).to.be.equal(errorToThrow);
  });

  it('should emit ERROR event in case SpvChain fails to addHeaders', async () => {
    await blockHeadersProvider.start();

    const errorToThrow = new Error('test');
    spvChainMock.addHeaders.onFirstCall().throws(errorToThrow);

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    blockHeadersReaderMock.emit(BlockHeadersReader.EVENTS.BLOCK_HEADERS, []);

    expect(emittedError).to.be.equal(errorToThrow);
  });

  it('should emit ERROR event in case of a failure subscribing to the new block headers', async () => {
    await blockHeadersProvider.start();

    const errorToThrow = new Error('test');
    blockHeadersReaderMock.subscribeToNew.onFirstCall().rejects(errorToThrow);

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    blockHeadersReaderMock.emit(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED);

    await sleepOneTick();

    expect(emittedError).to.be.equal(errorToThrow);
  });
});
