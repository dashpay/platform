const stream = require('stream');

const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');

const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));
const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersProvider', () => {
  let options;

  let coreApiMock;
  let blockHeadersReader;
  let blockHeadersStream;
  const mockedHeaders = getHeadersFixture();
  const headersBatchSize = 5;

  beforeEach(function () {
    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
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
              return;
            }

            start = fromBlockHeight - 1;
          }

          let end = start + headersBatchSize;
          end = end > lastItemIndex ? lastItemIndex : end;

          const headersToReturn = mockedHeaders.slice(start, end);

          // Simulate async emission
          await sleepOneTick();

          this.push({
            getBlockHeaders: () => ({
              getHeadersList: () => headersToReturn,
            }),
          });

          start = end;
        },
        objectMode: true,
      });
      return blockHeadersStream;
    });

    options = {
      coreMethods: coreApiMock,
      maxRetries: 5,
      maxParallelStreams: 6,
      targetBatchSize: 10,
    };

    blockHeadersReader = new BlockHeadersReader(options);
  });

  afterEach(() => {
    if (blockHeadersStream) {
      blockHeadersStream.destroy();
    }
  });

  describe('#subscribeToHistoricalBatch', () => {
    let subscribeToHistoricalBatch;
    beforeEach(() => {
      subscribeToHistoricalBatch = blockHeadersReader
        .subscribeToHistoricalBatchHOF(options.maxRetries);
    });

    it('should deliver block headers from the stream', async () => {
      const count = Math.ceil(mockedHeaders.length / 2);

      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      await subscribeToHistoricalBatch(1, count);

      while (obtainedHeaders.length < count) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(obtainedHeaders).to.deep.equal(mockedHeaders.slice(0, count));
    });

    it('should deliver block headers in case of errors and retry attempts', async () => {
      let obtainedHeaders = [];

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      subscribeToHistoricalBatch(1, mockedHeaders.length);

      // Wait for the first chunk of data to enter the stream
      await sleepOneTick();

      for (let i = 0; i < Math.ceil(options.maxRetries / 2); i += 1) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
        blockHeadersStream.destroy(new Error());
      }

      while (obtainedHeaders.length !== mockedHeaders.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(obtainedHeaders).to.deep.equal(mockedHeaders);
    });
  });

  describe('#subscribeToNew', () => {
    beforeEach(async () => {
      await blockHeadersReader.subscribeToNew(1);
    });

    it('should deliver block headers from the stream', async () => {
      let obtainedHeaders = null;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = headers;
        blockHeadersStream.destroy();
      });

      while (!obtainedHeaders) {
        // eslint-disable-next-line no-await-in-loop
        await sleep(100);
      }

      expect(obtainedHeaders).to.deep.equal(mockedHeaders.slice(0, headersBatchSize));
    });
  });

  describe('#readHistorical', () => {
    it('should read all historical block headers', async () => {
      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      await blockHeadersReader.readHistorical(1, mockedHeaders.length);

      while (obtainedHeaders.length !== mockedHeaders.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      obtainedHeaders.sort((a, b) => a.timestamp - b.timestamp);
      expect(obtainedHeaders).to.deep.equal(mockedHeaders);
    });
  });
});
