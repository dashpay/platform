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

  describe('#fetchBatch', () => {
    let fetchBatch;
    beforeEach(() => {
      fetchBatch = blockHeadersReader.createBatchFetcher();
    });

    it('should emit BLOCK_HEADERS event', async () => {
      const count = Math.ceil(mockedHeaders.length / 2);

      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      await fetchBatch(1, count);

      expect(obtainedHeaders).to.deep.equal(mockedHeaders.slice(0, count));
    });

    it('should deliver all block headers in case of errors and retry attempts', async () => {
      let obtainedHeaders = [];
      let completed = false;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      fetchBatch(1, mockedHeaders.length).then(() => {
        completed = true;
      });

      // Wait for the first chunk of data to enter the stream
      await sleepOneTick();

      for (let i = 0; i < Math.ceil(options.maxRetries / 2); i += 1) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
        blockHeadersStream.destroy(new Error());
      }

      while (!completed) {
        // eslint-disable-next-line no-await-in-loop
        await sleep(100);
      }

      expect(obtainedHeaders).to.deep.equal(mockedHeaders);
    });

    it('should throw an error in case the amount of retry attempts reached it\'s limit', async () => {
      const errorToThrow = new Error('');
      let errorThrown;
      fetchBatch(1, mockedHeaders.length).catch((e) => {
        errorThrown = e;
      });

      for (let i = 0; i < options.maxRetries + 1; i += 1) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
        blockHeadersStream.destroy(errorToThrow);
      }

      while (!errorThrown) {
        // eslint-disable-next-line no-await-in-loop
        await new Promise((resolve) => setTimeout(resolve, 100));
      }

      expect(errorThrown).to.equal(errorToThrow);
    });
  });

  describe('#subscribeToNew', () => {
    beforeEach(async () => {
      await blockHeadersReader.subscribeToNew(1);
    });

    it('should emit BLOCK_HEADERS event', async () => {
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

    it('should propagate an error from the stream', async () => {
      const errorToThrow = new Error();
      let errorThrown = null;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        errorThrown = e;
      });

      blockHeadersStream.destroy(errorToThrow);

      while (!errorThrown) {
        // eslint-disable-next-line no-await-in-loop
        await sleep(100);
      }

      expect(errorThrown).to.equal(errorToThrow);
    });
  });

  describe('#readHistorical', () => {
    it('should read all historical block headers', async () => {
      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      await blockHeadersReader.readHistorical(1, mockedHeaders.length);

      obtainedHeaders.sort((a, b) => a.timestamp - b.timestamp);
      expect(obtainedHeaders).to.deep.equal(mockedHeaders);
    });
  });
});
