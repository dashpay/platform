const stream = require('stream');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');
const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');

const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));
const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersProvider - integration', () => {
  let coreApiMock;
  let blockHeadersProvider;
  let blockHeadersStream;
  const mockedHeaders = getHeadersFixture();

  beforeEach(function () {
    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
      getStatus: this.sinon.stub().resolves({
        chain: {
          blocksCount: Math.ceil(mockedHeaders.length / 2),
        },
      }),
    };

    this.sinon.stub(coreApiMock, 'subscribeToBlockHeadersWithChainLocks').callsFake(async (args) => {
      const { fromBlockHeight, count } = args;
      let start = fromBlockHeight - 1;

      const lastItemIndex = count
        ? start + count : mockedHeaders.length;

      blockHeadersStream = new stream.Readable({
        async read() {
          if (start >= lastItemIndex) {
            if (count) {
              this.push(null);
            }

            // Stop emission here
            return;
          }

          const headersToReturn = mockedHeaders.slice(start, lastItemIndex);

          // Simulate async emission
          await sleepOneTick();

          this.push({
            getBlockHeaders: () => ({
              getHeadersList: () => headersToReturn.map((header) => header.toBuffer()),
            }),
          });

          start = lastItemIndex;
        },
        objectMode: true,
      });

      return blockHeadersStream;
    });

    blockHeadersProvider = new BlockHeadersProvider();
    blockHeadersProvider.setCoreMethods(coreApiMock);
  });

  afterEach(() => {
    if (blockHeadersStream) {
      blockHeadersStream.destroy();
    }
  });

  it('should obtain all block headers and validate them against the SPV chain', async () => {
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

  it('should retry to obtain historical headers in case of SPV failure', async () => {
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
    blockHeadersProvider.blockHeadersReader
      .emit(BlockHeadersProvider.EVENTS.ERROR, errorToThrow);

    expect(emittedError).to.be.equal(errorToThrow);
  });

  it('should emit ERROR event in case SpvChain fails to addHeaders', async function () {
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

  it('should emit ERROR event in case of a failure subscribing to the new block headers', async function () {
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
