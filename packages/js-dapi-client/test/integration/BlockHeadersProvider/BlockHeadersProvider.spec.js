const stream = require('stream');
const mockedHeaders = require('./headers');
const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider');

const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));

describe('BlockHeadersProvider', () => {
  let coreApiMock;
  let blockHeadersProvider;
  let blockHeadersStream;

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
          await new Promise((resolve) => setImmediate(resolve));

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
});
