const EventEmitter = require('events');
const { expect } = require('chai');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');

const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersReader', () => {
  let options;

  let blockHeadersReader;
  let streamMock;
  let subscribeToBlockHeadersWithChainLocksStub;
  beforeEach(function () {
    const coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
    };

    const { sinon } = this;
    subscribeToBlockHeadersWithChainLocksStub = sinon.stub(coreApiMock, 'subscribeToBlockHeadersWithChainLocks')
      .callsFake(() => {
        streamMock = new EventEmitter();
        streamMock.destroy = (e) => {
          if (e) {
            streamMock.emit('error', e);
          }
        };

        sinon.spy(streamMock, 'on');

        return streamMock;
      });

    options = {
      coreMethods: coreApiMock,
      maxRetries: 0,
      maxParallelStreams: 6,
      targetBatchSize: 10,
    };

    blockHeadersReader = new BlockHeadersReader(options);
  });

  describe('#subscribeToNew', () => {
    beforeEach(async () => {
      await blockHeadersReader.subscribeToNew(1);
    });

    it('should emit BLOCK_HEADERS event', () => {
      const headersToSend = ['0xdeadbeef', '0xbeefdead'];
      let headersFromEvent;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = headers;
      });

      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => headersToSend,
        }),
      });

      expect(headersFromEvent).to.deep.equal(headersToSend);
    });

    it('should emit ERROR event in case of the stream error', () => {
      let emittedError;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        emittedError = e;
      });
      const errorToThrow = new Error('test');
      streamMock.emit('error', errorToThrow);
      expect(emittedError).to.equal(errorToThrow);
    });

    it('should emit ERROR event in case of deliberate rejection of BLOCK_HEADERS', () => {
      const errorToRejectWith = new Error('test');
      let errorEmitted;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        errorEmitted = e;
      });

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (_, reject) => {
        // Simulate rejection of the headers in case they are not valid
        reject(errorToRejectWith);
      });

      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => [],
        }),
      });

      expect(errorEmitted).to.equal(errorToRejectWith);
    });
  });

  describe('#subscribeToHistoricalBatch', () => {
    it('should emit BLOCK_HEADERS event', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

      const headersToSend = ['0xdeadbeef', '0xbeefdead'];
      let headersFromEvent;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = headers;
      });

      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => headersToSend,
        }),
      });

      expect(headersFromEvent).to.deep.equal(headersToSend);
    });

    it('should emit BLOCK_HEADERS event in case of error and retry attempt', async () => {
      const maxRetries = 3;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

      const headersToSend = ['0xdeadbeef', '0xbeefdead'];
      let headersFromEvent;

      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        headersFromEvent = headers;
      });

      let emittedError;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        emittedError = e;
      });

      for (let i = 0; i < maxRetries; i += 1) {
        streamMock.emit('error', new Error('test'));
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => headersToSend,
        }),
      });

      expect(emittedError).to.not.exist();
      expect(headersFromEvent).to.deep.equal(headersToSend);
    });

    it('should emit HANDLE_STREAM_ERROR command in case of the stream error', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

      let emittedError;
      let streamFromCommand;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        streamFromCommand = stream;
        emittedError = e;
      });
      const errorToThrow = new Error('test');
      streamMock.emit('error', errorToThrow);
      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(streamMock);
    });

    it('should emit HANDLE_STREAM_ERROR command in case of deliberate rejection of the headers', async () => {
      const maxRetries = 0;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

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

      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => [],
        }),
      });

      expect(errorEmitted).to.equal(errorToRejectWith);
      expect(streamFromCommand).to.equal(streamMock);
    });

    it('should emit HANDLE_STREAM_ERROR command if retry attempts are exhausted', async () => {
      const maxRetries = 3;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

      const errorToThrow = new Error('test');
      let emittedError;
      let streamFromCommand;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
        emittedError = e;
        streamFromCommand = stream;
      });

      for (let i = 0; i < maxRetries + 1; i += 1) {
        streamMock.emit('error', errorToThrow);
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(streamMock);
    });

    it('should emit HANDLE_STREAM_ERROR if stream failed to be created in retry attempt', async () => {
      const maxRetries = 1;
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        maxRetries,
      );
      await subscribeToHistoricalBatch(1, 1);

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
      streamMock.emit('error', new Error('test'));

      await sleepOneTick();
      expect(emittedError).to.equal(errorToThrow);
      expect(streamFromCommand).to.equal(streamMock);
    });
  });

  describe('#readHistorical', () => {
    let headersAmount;
    beforeEach(async () => {
      headersAmount = options.targetBatchSize * 5;
      await blockHeadersReader.readHistorical(1, headersAmount);
    });

    it('should emit BLOCK_HEADERS event', () => {
      let obtainedHeaders = [];
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (headers) => {
        obtainedHeaders = [...obtainedHeaders, ...headers];
      });

      const headers = Array.from({ length: headersAmount })
        .map((_, i) => `0x${i.toString(16)}`);

      blockHeadersReader.historicalStreams.forEach((stream, index) => {
        stream.emit('data', {
          getBlockHeaders: () => ({
            getHeadersList: () => {
              const startIndex = index * options.targetBatchSize;
              const endIndex = startIndex + options.targetBatchSize;
              return headers.slice(startIndex, endIndex);
            },
          }),
        });
      });

      expect(obtainedHeaders).to.deep.equal(headers);
    });

    it('should emit HISTORICAL_DATA_OBTAINED event once all historical block headers fetched', () => {
      let eventEmitted = false;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
        eventEmitted = true;
      });

      [...blockHeadersReader.historicalStreams].forEach((stream) => {
        stream.emit('end');
      });

      expect(eventEmitted).to.be.true();
    });

    it('should emit ERROR event in case of errors in streams', () => {
      const errorsEmitted = [];
      const errorsToEmit = [];

      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        errorsEmitted.push(e);
      });

      [...blockHeadersReader.historicalStreams].forEach((stream, i) => {
        const e = new Error(`test${i}`);
        errorsToEmit.push(e);
        stream.emit('error', e);
      });

      expect(errorsEmitted).to.deep.equal(errorsToEmit);
    });
  });
});
