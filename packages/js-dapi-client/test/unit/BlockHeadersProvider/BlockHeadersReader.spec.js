const { expect } = require('chai');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');
const BlockHeadersWithChainLocksStreamMock = require('../../../lib/test/mocks/BlockHeadersWithChainLocksStreamMock');

const sleepOneTick = () => new Promise((resolve) => {
  if (typeof setImmediate === 'undefined') {
    setTimeout(resolve, 10);
  } else {
    setImmediate(resolve);
  }
});

describe('BlockHeadersReader - unit', () => {
  let options;

  let blockHeadersReader;
  let historicalStreams;
  let continuousSyncStream;

  beforeEach(function () {
    historicalStreams = [];

    options = {
      maxRetries: 1,
      maxParallelStreams: 6,
      targetBatchSize: 10,
    };

    const createHistoricalSyncStream = this.sinon.stub()
      .callsFake(async () => {
        const stream = new BlockHeadersWithChainLocksStreamMock(this.sinon);
        historicalStreams.push(stream);
        return stream;
      });
    const createContinuousSyncStream = this.sinon.stub()
      .callsFake(async () => {
        const stream = new BlockHeadersWithChainLocksStreamMock(this.sinon);
        continuousSyncStream = stream;
        return stream;
      });

    blockHeadersReader = new BlockHeadersReader(
      options, createHistoricalSyncStream, createContinuousSyncStream,
    );
    this.sinon.spy(blockHeadersReader, 'emit');
    this.sinon.spy(blockHeadersReader, 'on');
    this.sinon.spy(blockHeadersReader, 'stopReadingHistorical');
  });

  describe('#createSubscribeToHistoricalBatch', () => {
    let headers;
    let subscribeToHistoricalBatch;

    beforeEach(() => {
      headers = getHeadersFixture();

      subscribeToHistoricalBatch = blockHeadersReader.createSubscribeToHistoricalBatch(
        options.maxRetries,
      );
    });

    it('should subscribe to block headers stream and hook on events', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      expect(blockHeadersReader.createHistoricalSyncStream)
        .to.have.been.calledWith(1, headers.length);

      const stream = historicalStreams[0];
      expect(stream.on).to.have.been.calledWith('data');
      expect(stream.on).to.have.been.calledWith('error');
      expect(stream.on).to.have.been.calledWith('end');
    });

    it('[data] process headers batch', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];
      stream.sendHeaders(headers);

      const { firstCall } = blockHeadersReader.emit;
      expect(blockHeadersReader.emit).to.have.been.calledOnce();
      expect(firstCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(firstCall.args[1]).to.deep.equal({
        headers,
        headHeight: 1,
      });
    });

    it('[data] should provide correct batch head height for every emitted batch', async () => {
      const startFrom = 2;

      await subscribeToHistoricalBatch(startFrom, headers.length);
      const stream = historicalStreams[0];

      // Send first batch
      const firstBatch = headers.slice(0, headers.length / 2);
      stream.sendHeaders(firstBatch);

      // Send second batch
      const secondBatch = headers.slice(headers.length / 2);
      stream.sendHeaders(secondBatch);

      const { firstCall, secondCall } = blockHeadersReader.emit;
      expect(firstCall.args[1]).to.deep.equal({
        headers: firstBatch,
        headHeight: startFrom,
      });

      expect(secondCall.args[1]).to.deep.equal({
        headers: secondBatch,
        headHeight: startFrom + firstBatch.length,
      });
    });

    it('[data] should cancel and retry stream in case headers batch was rejected', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const rejectWith = new Error('Invalid headers');
      const rejectionPromise = new Promise((resolve) => {
        blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (_, rejecHeaders) => {
          rejecHeaders(rejectWith);
          resolve();
        });
      });

      const stream = historicalStreams[0];
      stream.sendHeaders(headers);

      await rejectionPromise;

      expect(stream.cancel).to.have.been.calledOnce();
      expect(blockHeadersReader.createHistoricalSyncStream)
        .to.have.been.calledTwice();
    });

    it('[error] should handle stream cancellation', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];
      stream.emit('error', {
        code: GrpcErrorCodes.CANCELLED,
      });

      await sleepOneTick();

      // Ensure "return" behaviour
      expect(blockHeadersReader.emit).to.have.not.been.called();
      expect(blockHeadersReader.createHistoricalSyncStream)
        .to.have.been.calledOnce();
    });

    it('[error] should retry in case of an error', async () => {
      const startFrom = 1;

      await subscribeToHistoricalBatch(startFrom, headers.length);
      const originalStream = historicalStreams[0];

      const firstBatch = headers.slice(0, headers.length / 3);
      originalStream.sendHeaders(firstBatch);

      // Emit error
      originalStream.emit('error', new Error('Fake stream error'));

      await sleepOneTick();

      const secondBatch = headers.slice(headers.length / 3);
      originalStream.sendHeaders(secondBatch);

      const { firstCall, secondCall } = blockHeadersReader.emit;

      // Ensure first batch was emitted
      expect(firstCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(firstCall.args[1]).to.deep.equal({
        headers: firstBatch,
        headHeight: 1,
      });

      // Ensure that retry logic fetches only headers that weren't processed yet
      const fromBlock = startFrom + firstBatch.length;
      const count = headers.length - firstBatch.length;
      expect(blockHeadersReader.createHistoricalSyncStream.secondCall.args)
        .to.deep.equal([fromBlock, count]);

      // Ensure that second batch was emitted
      expect(secondCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(secondCall.args[1]).to.deep.equal({
        headers: secondBatch,
        headHeight: startFrom + firstBatch.length,
      });
    });

    it('[error] should emit error in case of resubscribe failure', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];

      const resubscribeError = new Error('Error subscribing to block headers');

      // Prepare subscribe function to throw an error on retry attempt
      blockHeadersReader.createHistoricalSyncStream.throws(resubscribeError);

      // Emit stream error to trigger retry attempt
      stream.emit('error', new Error('Invalid block headers'));

      await sleepOneTick();

      expect(blockHeadersReader.emit).to.have.been.calledOnce();
      expect(blockHeadersReader.stopReadingHistorical).to.have.been.calledOnce();
    });

    it('[error] should emit error in case retry attempts were exhausted', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];

      const firstError = new Error('firstError');
      stream.emit('error', firstError);

      await sleepOneTick();

      blockHeadersReader.on('error', () => {});
      const secondError = new Error('secondError');
      stream.emit('error', secondError);

      expect(blockHeadersReader.emit)
        .to.have.been.calledWith('error', secondError);
    });

    it('[end] should handle end event', async () => {
      await subscribeToHistoricalBatch(1, headers.length);
      const stream = historicalStreams[0];

      stream.emit('end');

      expect(blockHeadersReader.emit)
        .to.have.been.calledOnceWithExactly(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED);
    });
  });

  describe('#subscribeToNew', () => {
    const startFrom = 100;
    let headers;
    beforeEach(async () => {
      headers = getHeadersFixture();
    });

    it('should subscribe to block headers stream and hook on events', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      expect(blockHeadersReader.createContinuousSyncStream)
        .to.have.been.calledWith(startFrom);

      expect(continuousSyncStream.on).to.have.been.calledWith('data');
      expect(continuousSyncStream.on).to.have.been.calledWith('error');
      expect(continuousSyncStream.on).to.have.been.calledWith('end');
      expect(continuousSyncStream.on).to.have.been.calledWith('beforeReconnect');
    });

    it('should validate fromBlockHeight', async () => {
      await expect(blockHeadersReader.subscribeToNew(-1))
        .to.be.rejectedWith('Invalid fromBlockHeight: -1');
    });

    it('should not allow subscribe twice', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      await expect(blockHeadersReader.subscribeToNew(-1))
        .to.be.rejectedWith('Continuous sync has already been started');
    });

    it('[data] should process headers batch', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      const headersToSend = headers.slice(0, 2);
      continuousSyncStream.sendHeaders(headersToSend);

      expect(blockHeadersReader.emit).to.have.been.calledOnce();
      expect(blockHeadersReader.emit).to.have.been.calledWith(
        BlockHeadersReader.EVENTS.BLOCK_HEADERS,
        {
          headers: headersToSend,
          headHeight: startFrom,
        },
      );
    });

    it('[data] should provide correct head height for every emitted batch', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      const firstBatch = headers.slice(0, 2);
      continuousSyncStream.sendHeaders(firstBatch);

      const newHeader = headers.slice(2, 3);
      continuousSyncStream.sendHeaders(newHeader);

      const { firstCall, secondCall } = blockHeadersReader.emit;
      expect(blockHeadersReader.emit).to.have.been.calledTwice();

      expect(firstCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(firstCall.args[1]).to.deep.equal(
        {
          headers: firstBatch,
          headHeight: startFrom,
        },
      );

      expect(secondCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(secondCall.args[1]).to.deep.equal(
        {
          headers: newHeader,
          headHeight: startFrom + 2,
        },
      );
    });

    it('[data] should cancel stream in case headers batch was rejected', async function () {
      await blockHeadersReader.subscribeToNew(startFrom);

      continuousSyncStream.retryOnError = this.sinon.stub().callsFake((e) => {
        continuousSyncStream.emit('error', e);
      });

      const rejectWith = new Error('Stream error');
      blockHeadersReader.on(BlockHeadersReader.EVENTS.BLOCK_HEADERS, (_, rejectHeaders) => {
        rejectHeaders(rejectWith);
      });

      const errorPromise = new Promise((resolve) => {
        blockHeadersReader.on('error', resolve);
      });

      continuousSyncStream.sendHeaders(headers);

      const emittedError = await errorPromise;

      expect(emittedError).to.equal(rejectWith);
    });

    it('[beforeReconnect] should maintain correct lastKnownChainHeight before reconnect happens', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      // Emit first batch
      const firstBatch = headers.slice(0, 2);
      continuousSyncStream.sendHeaders(firstBatch);

      // Trigger stream reconnect
      const beforeReconnectPromise = new Promise((resolve) => {
        continuousSyncStream.emit('beforeReconnect', (updatedArgs) => {
          resolve(updatedArgs);
        });
      });

      const reconnectArgs = await beforeReconnectPromise;

      // Emit header after reconnect
      const newHeaders = headers.slice(1, 3);
      continuousSyncStream.sendHeaders(newHeaders);

      // Ensure reconnect args adjusted correctly
      expect(reconnectArgs.fromBlockHeight).to.equal(startFrom + 1);

      const { firstCall, secondCall } = blockHeadersReader.emit;
      expect(blockHeadersReader.emit).to.have.been.calledTwice();

      // Ensure correct headers were emitted in first event
      let emittedHeaders = firstCall.args[1].headers.map((header) => header.toObject());
      let expectedHeaders = firstBatch.map((header) => header.toObject());
      expect(firstCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(emittedHeaders).to.deep.equal(expectedHeaders);
      expect(firstCall.args[1].headHeight).to.equal(startFrom);

      // Ensure correct headers were emitted in second event
      emittedHeaders = secondCall.args[1].headers.map((header) => header.toObject());
      expectedHeaders = newHeaders.map((header) => header.toObject());
      expect(secondCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(emittedHeaders).to.deep.equal(expectedHeaders);
      expect(secondCall.args[1].headHeight).to.equal(startFrom + 1);
    });

    it('[error] should handle stream cancellation', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      continuousSyncStream.emit('error', {
        code: 1,
      });

      expect(blockHeadersReader.continuousSyncStream).to.equal(null);
      expect(blockHeadersReader.emit).to.have.not.been.called();
    });

    it('[error] should handle stream error', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);

      const errorPromise = new Promise((resolve) => {
        blockHeadersReader.on('error', resolve);
      });

      const error = new Error('Stream error');
      continuousSyncStream.emit('error', error);

      expect(blockHeadersReader.continuousSyncStream).to.equal(null);
      const emittedError = await errorPromise;
      expect(emittedError).to.equal(error);
    });

    it('[end] should handle end event', async () => {
      await blockHeadersReader.subscribeToNew(startFrom);
      continuousSyncStream.emit('end');
      expect(blockHeadersReader.continuousSyncStream).to.equal(null);
    });
  });

  describe('#unsubscribeFromNew', () => {
    it('should unsubscribe from new headers', async () => {
      await blockHeadersReader.subscribeToNew(1);
      const { continuousSyncStream: stream } = blockHeadersReader;
      blockHeadersReader.unsubscribeFromNew();
      expect(stream.cancel).to.have.been.calledOnce();
      expect(blockHeadersReader.continuousSyncStream).to.equal(null);
    });
  });

  describe('#readHistorical', () => {
    beforeEach(function () {
      this.sinon.spy(blockHeadersReader, 'createSubscribeToHistoricalBatch');
    });

    it('should start historical sync and subscribe to events and commands', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize);

      expect(blockHeadersReader.historicalStreams.length).to.equal(1);
      expect(blockHeadersReader.createHistoricalSyncStream).to.have.been.calledOnceWith(
        1,
        options.targetBatchSize,
      );
    });

    it('should create only one stream in case the amount of blocks is too small', async () => {
      const amount = Math.ceil(options.targetBatchSize * 1.4);
      await blockHeadersReader.readHistorical(1, amount);
      expect(blockHeadersReader.createSubscribeToHistoricalBatch).to.be.calledOnce();
    });

    it('should evenly spread the load between streams', async () => {
      const fromBlock = 1;
      const toBlock = Math.round(options.targetBatchSize * 3.5 - 1);
      const totalAmount = toBlock - fromBlock + 1;
      const numStreams = Math.round(totalAmount / options.targetBatchSize);

      const itemsPerBatch = Math.ceil(totalAmount / numStreams);

      await blockHeadersReader.readHistorical(fromBlock, toBlock);

      const { createHistoricalSyncStream } = blockHeadersReader;
      expect(createHistoricalSyncStream).to.be.calledThrice();
      expect(createHistoricalSyncStream.getCall(0).args)
        .to.deep.equal([fromBlock, itemsPerBatch]);
      expect(createHistoricalSyncStream.getCall(1).args)
        .to.deep.equal([fromBlock + itemsPerBatch, itemsPerBatch]);
      expect(createHistoricalSyncStream.getCall(2).args)
        .to.deep.equal([fromBlock + 2 * itemsPerBatch, totalAmount - itemsPerBatch * 2]);
    });

    it('should limit amount of streams in case batch size is too small compared to total amount', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 10);
      expect(blockHeadersReader.createSubscribeToHistoricalBatch.callCount)
        .to.equal(options.maxParallelStreams);
    });

    it('should throw an error in case the total amount of headers is less than 1', async () => {
      await expect(blockHeadersReader.readHistorical(2, 1))
        .to.be.rejectedWith('Invalid total amount of headers to read: 0');
    });

    it('should throw an error if fromBlockHeight is less than 1', async () => {
      await expect(blockHeadersReader.readHistorical(0, 3))
        .to.be.rejectedWith('Invalid fromBlockHeight value: 0');
    });

    it('should not allow multiple executions', async () => {
      await blockHeadersReader.readHistorical(1, 3);
      await expect(blockHeadersReader.readHistorical(1, 3))
        .to.be.rejectedWith('Historical streams are already running. Please stop them first.');
    });

    it('should replace stream in historicalStreams in case of retry attempt', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 5);

      const streamToBreak = blockHeadersReader.historicalStreams[3];
      streamToBreak.emit('error', new Error('retry'));

      await sleepOneTick();

      expect(blockHeadersReader.historicalStreams.includes(streamToBreak)).to.be.false();
    });

    it('should remove stream from historicalStreams array in case of end', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);
      const streamsAmount = blockHeadersReader.historicalStreams.length;

      const streamToFinish = blockHeadersReader.historicalStreams[0];
      streamToFinish.emit('end');

      expect(blockHeadersReader.historicalStreams.length).to.equal(streamsAmount - 1);
      expect(blockHeadersReader.historicalStreams.includes(streamToFinish)).to.be.false();
    });

    it('should dispatch HISTORICAL_DATA_OBTAINED in case all streams have ended', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);

      [...blockHeadersReader.historicalStreams].forEach((stream) => stream.emit('end'));
      expect(blockHeadersReader.emit)
        .to.have.been.calledWith(BlockHeadersReader.EVENTS.HISTORICAL_DATA_OBTAINED);
    });

    it('should remove stream from historicalStreams array in case of cancellation', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);
      const streamsAmount = blockHeadersReader.historicalStreams.length;

      const streamToFinish = blockHeadersReader.historicalStreams[0];
      streamToFinish.emit('error', {
        code: 1,
      });

      expect(blockHeadersReader.historicalStreams.length).to.equal(streamsAmount - 1);
      expect(blockHeadersReader.historicalStreams.includes(streamToFinish)).to.be.false();
    });

    it('should stop reading historical data in case of one of the streams throws an error', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);

      const errorPromise = new Promise((resolve) => {
        blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, resolve);
      });

      let lastError;
      // Exhaust all retry attempts to actually throw an error
      for (let i = 0; i < options.maxRetries + 1; i += 1) {
        const [streamToBreak] = blockHeadersReader.historicalStreams;
        lastError = new Error('retry');
        streamToBreak.emit('error', lastError);
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      const emittedError = await errorPromise;
      expect(emittedError).to.equal(lastError);
      expect(blockHeadersReader.stopReadingHistorical).to.have.been.calledOnce();
    });
  });

  describe('#stopReadingHistorical', async () => {
    it('should stop reading historical data and unsubscribe from all events', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);
      expect(blockHeadersReader.historicalStreams[1]).to.equal(historicalStreams[1]);
      blockHeadersReader.stopReadingHistorical();
      historicalStreams.forEach((stream) => {
        expect(stream.cancel).to.have.been.called();
      });
      expect(blockHeadersReader.historicalStreams).to.have.length(0);
    });
  });
});
