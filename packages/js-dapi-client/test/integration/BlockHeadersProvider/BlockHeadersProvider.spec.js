const { BlockHeader } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');
const BlockHeadersWithChainLocksStreamMock = require('../../../lib/test/mocks/BlockHeadersWithChainLocksStreamMock');
const mockHeadersChain = require('../../../lib/test/mocks/mockHeadersChain');

const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));
const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersProvider - integration', () => {
  // let coreApiMock;
  let blockHeadersProvider;
  let historicalStreams = [];
  let continuousStream;
  let sandbox;

  const createBlockHeadersProvider = (sinon, opts = {}) => {
    historicalStreams = [];
    continuousStream = null;

    const subscribeToBlockHeadersWithChainLocks = sinon.stub();

    subscribeToBlockHeadersWithChainLocks
      .callsFake(({ count }) => {
        const stream = new BlockHeadersWithChainLocksStreamMock(sinon);
        if (count > 0) {
          historicalStreams.push(stream);
        } else {
          continuousStream = stream;
        }

        return stream;
      });

    blockHeadersProvider = new BlockHeadersProvider(opts);

    blockHeadersProvider.setCoreMethods({
      subscribeToBlockHeadersWithChainLocks,
    });
  };

  describe('#readHistorical', () => {
    // Start from height bigger than the first block
    // because we need to make sure that spv chain could bootstrap itself
    // from any header
    const startFrom = 10;
    let headers;

    before(async function () {
      const numHeaders = 500;
      headers = mockHeadersChain('testnet', numHeaders);

      createBlockHeadersProvider(this.sinon, {
        targetBatchSize: numHeaders / 5,
      });
    });

    beforeEach(function () {
      this.sinon.spy(blockHeadersProvider, 'emit');
    });

    it('it should add batches from the tail', async () => {
      await blockHeadersProvider.readHistorical(startFrom, startFrom + headers.length);

      historicalStreams.forEach((stream, i) => {
        if (i !== 0) {
          stream.sendHeaders(
            headers.slice(i * (headers.length / 5), (i + 1) * (headers.length / 5)),
          );
          stream.destroy();
        }
      });

      const { spvChain } = blockHeadersProvider;

      // Headers added from the tail should be orphaned
      expect(spvChain.getOrphanChunks()).to.have.length(4);
      expect(spvChain.getLongestChain()).to.have.length(0);
      expect(blockHeadersProvider.emit.callCount).to.equal(4);
      expect(blockHeadersProvider.emit).to
        .have.been.calledWith(BlockHeadersProvider.EVENTS.CHAIN_UPDATED);
    });

    it('should add batch from the head and form longest chain', async () => {
      historicalStreams[0].sendHeaders(headers.slice(0, headers.length / 5));
      historicalStreams[0].destroy();

      const { spvChain } = blockHeadersProvider;
      expect(spvChain.getLongestChain({ withPruned: true }))
        .to.have.length(headers.length);
      expect(blockHeadersProvider.emit).to
        .have.been.calledWith(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
    });
  });

  describe('#startContinuousSync', () => {

  });

  it.skip('should obtain all block headers and validate them against the SPV chain', async () => {
    await blockHeadersProvider.start();

    let longestChain = blockHeadersProvider.spvChain.getLongestChain();

    while (longestChain.length !== mockedHeaders.length + 1) {
      // eslint-disable-next-line no-await-in-loop
      await sleep(100);
      longestChain = blockHeadersProvider.spvChain.getLongestChain();
    }

    // slice(1): ignore genesis block
    expect(longestChain.slice(1).map((header) => header.hash))
      .to.deep.equal(mockedHeaders.map((header) => header.hash));
  });

  it.skip('should retry to obtain historical headers in case of SPV failure', async () => {
    blockHeadersProvider.start();

    await sleepOneTick();

    // Perform MITM attack :)
    const badHeader = mockedHeaders[0].toObject();
    delete badHeader.hash;
    badHeader.prevHash = Buffer.from('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc22', 'hex');

    blockHeadersStream.push({
      getBlockHeaders: () => ({
        getHeadersList: () => [new BlockHeader(badHeader).toBuffer()],
      }),
    });

    // Continue waiting for the recovery
    let longestChain = blockHeadersProvider.spvChain.getLongestChain();

    while (longestChain.length !== mockedHeaders.length + 1) {
      // eslint-disable-next-line no-await-in-loop
      await sleep(100);
      longestChain = blockHeadersProvider.spvChain.getLongestChain();
    }

    // slice(1): ignore genesis block
    expect(longestChain.slice(1).map((header) => header.hash))
      .to.deep.equal(mockedHeaders.map((header) => header.hash));
  });

  it.skip('should throw error in case core methods are missing', async () => {
    blockHeadersProvider.setCoreMethods(null);
    try {
      await blockHeadersProvider.start();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
    }
  });

  it.skip('should throw error in case BlockHeadersProvider has already been started', async () => {
    await blockHeadersProvider.start();

    try {
      await blockHeadersProvider.start();
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
    }
  });

  it.skip('should emit ERROR event in case BlockHeadersReader emits ERROR', async () => {
    await blockHeadersProvider.start();

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    const errorToThrow = new Error('test');
    blockHeadersProvider.blockHeadersReader
      .emit(BlockHeadersProvider.EVENTS.ERROR, errorToThrow);

    expect(emittedError).to.be.equal(errorToThrow);
  });

  it.skip('should emit ERROR event in case SpvChain fails to addHeaders', async function () {
    const errorToThrow = new Error('test');
    blockHeadersProvider.spvChain.addHeaders = this.sinon.stub();
    blockHeadersProvider.spvChain.addHeaders.onFirstCall().throws(errorToThrow);

    await blockHeadersProvider.start();

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    blockHeadersProvider.blockHeadersReader
      .emit(BlockHeadersReader.EVENTS.BLOCK_HEADERS, []);

    expect(emittedError).to.be.equal(errorToThrow);
  });

  it.skip('should emit ERROR event in case of a failure subscribing to the new block headers', async function () {
    await blockHeadersProvider.start();

    const errorToThrow = new Error('test');
    blockHeadersProvider.blockHeadersReader.subscribeToNew = this.sinon.stub()
      .rejects(errorToThrow);

    let emittedError;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      emittedError = e;
    });

    blockHeadersProvider.blockHeadersReader
      .emit(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED);

    await sleepOneTick();

    expect(emittedError).to.be.equal(errorToThrow);
  });
});
