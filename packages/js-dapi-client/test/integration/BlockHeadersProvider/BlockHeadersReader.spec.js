const { Readable: ReadableStream } = require('stream');

const { expect } = require('chai');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');

const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersReader - integration', () => {
  let options;

  let coreApiMock;
  let blockHeadersReader;
  let blockHeadersStream;
  let subscribeToBlockHeadersWithChainLocksStub;
  const mockedHeaders = getHeadersFixture();
  const headersBatchSize = 5;

  beforeEach(function () {
    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
    };

    subscribeToBlockHeadersWithChainLocksStub = this.sinon.stub(coreApiMock, 'subscribeToBlockHeadersWithChainLocks')
      .callsFake(async (args) => {
        const { fromBlockHeight, count } = args;
        let start = fromBlockHeight - 1;

        const lastItemIndex = count
          ? start + count : mockedHeaders.length;

        blockHeadersStream = new ReadableStream({
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
      maxRetries: 0,
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
    it('should emit BLOCK_HEADERS event', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, mockedHeaders.length);

      let headersFromEvent = [];

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = [...headersFromEvent, ...headers];
      });

      while (headersFromEvent.length !== mockedHeaders.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(headersFromEvent).to.deep.equal(mockedHeaders);
    });

    it('should emit BLOCK_HEADERS event in case of error and retry attempt', async () => {
      const maxRetries = 3;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, mockedHeaders.length);

      let headersFromEvent = [];

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = [...headersFromEvent, ...headers];
      });

      let emittedError;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        emittedError = e;
      });

      for (let i = 0; i < maxRetries; i += 1) {
        // Sleep two ticks in a row to simulate an error after every emission
        // of the chunk of data

        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();

        blockHeadersStream.destroy(new Error());
      }

      while (headersFromEvent.length !== mockedHeaders.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(emittedError).to.not.exist();
      expect(headersFromEvent).to.deep.equal(mockedHeaders);
    });

    it('should emit HANDLE_STREAM_ERROR command in case of the stream error', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, 1);

      let emittedError;
      let streamFromCommand;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        streamFromCommand = stream;
        emittedError = e;
      });
      const errorToThrow = new Error('test');
      blockHeadersStream.destroy(errorToThrow);

      await sleepOneTick();

      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(blockHeadersStream);
    });

    it('should emit HANDLE_STREAM_ERROR command in case of deliberate rejection of the headers', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, 1);

      const errorToRejectWith = new Error('test');
      let errorEmitted;
      let streamFromCommand;

      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        errorEmitted = e;
        streamFromCommand = stream;
      });

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (_, reject) => {
        // Simulate rejection of the headers in case they are not valid
        reject(errorToRejectWith);
      });

      while (!errorEmitted) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(errorEmitted).to.equal(errorToRejectWith);
      expect(streamFromCommand).to.equal(blockHeadersStream);
    });

    it('should emit HANDLE_STREAM_ERROR command if retry attempts are exhausted', async () => {
      const maxRetries = 3;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, 1);

      const errorToThrow = new Error('test');
      let emittedError;
      let streamFromCommand;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        emittedError = e;
        streamFromCommand = stream;
      });

      for (let i = 0; i < maxRetries + 1; i += 1) {
        blockHeadersStream.destroy(errorToThrow);
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      while (!emittedError) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(blockHeadersStream);
    });

    it('should emit HANDLE_STREAM_ERROR command if stream failed to be created in retry attempt', async () => {
      const maxRetries = 1;
      const subscribeToHistoricalBatchWithRetry = blockHeadersReader.subscribeToHistoricalBatch(
        maxRetries,
      );
      await subscribeToHistoricalBatchWithRetry(1, 1);

      // Throw an error on a second call of subscribe
      const errorToThrow = new Error('test');
      subscribeToBlockHeadersWithChainLocksStub.onSecondCall().rejects(errorToThrow);

      let emittedError;
      let streamFromCommand;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        emittedError = e;
        streamFromCommand = stream;
      });

      // Emit error from stream to trigger retry attempt
      blockHeadersStream.destroy(new Error('test'));

      await sleepOneTick();
      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(blockHeadersStream);
    });
  });

  describe('#subscribeToNew', () => {
    beforeEach(async () => {
      await blockHeadersReader.subscribeToNew(1);
    });

    it('should emit BLOCK_HEADERS event', async () => {
      let headersFromEvent;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = headers;
        blockHeadersStream.destroy();
      });

      while (!headersFromEvent) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(headersFromEvent).to.deep.equal(mockedHeaders.slice(0, headersBatchSize));
    });

    it('should emit ERROR event in case of the stream error', async () => {
      let emittedError;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        emittedError = e;
      });
      const errorToThrow = new Error('test');
      blockHeadersStream.destroy(errorToThrow);

      while (!emittedError) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(emittedError).to.equal(errorToThrow);
    });

    it('should emit ERROR event in case of deliberate rejection of BLOCK_HEADERS', async () => {
      const errorToRejectWith = new Error('test');
      let errorEmitted;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        errorEmitted = e;
      });

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (_, reject) => {
        // Simulate rejection of the headers in case they are not valid
        reject(errorToRejectWith);
      });

      while (!errorEmitted) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(errorEmitted).to.equal(errorToRejectWith);
    });
  });

  describe('#readHistorical', () => {
    let headersAmount;
    beforeEach(async () => {
      headersAmount = mockedHeaders.length;
      await blockHeadersReader.readHistorical(1, headersAmount);
    });

    // after

    it('should emit BLOCK_HEADERS event', async () => {
      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      while (obtainedHeaders.length !== mockedHeaders.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      obtainedHeaders.sort((a, b) => a.timestamp - b.timestamp);
      expect(obtainedHeaders).to.deep.equal(mockedHeaders);
    });

    it('should emit HISTORICAL_DATA_OBTAINED event once all historical block headers fetched', async () => {
      let eventEmitted;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
        eventEmitted = true;
      });

      while (typeof eventEmitted === 'undefined') {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(eventEmitted).to.be.true();
    });

    it('should emit ERROR event in case of errors in streams', async () => {
      const errorsEmitted = [];
      const errorsToEmit = [];

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        errorsEmitted.push(e);
      });

      [...blockHeadersReader.historicalStreams].forEach((stream, i) => {
        const e = new Error(`test${i}`);
        errorsToEmit.push(e);
        stream.destroy(e);
      });

      while (errorsEmitted.length !== errorsToEmit.length) {
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(errorsEmitted).to.deep.equal(errorsToEmit);
    });

    it('should throw an error in attempt to run readHistorical for a second time', async () => {
      try {
        await blockHeadersReader.readHistorical(1, headersAmount);
      } catch (e) {
        expect(e.message).to.equal('Historical streams are already running');
      }
    });
  });
});
