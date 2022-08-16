// const EventEmitter = require('events');
const { expect } = require('chai');
// const { BlockHeader } = require('@dashevo/dashcore-lib');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');
const getHeadersFixture = require('../../../lib/test/fixtures/getHeadersFixture');
// const DAPIStream = require('../../../lib/transport/DAPIStream');
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
      createHistoricalSyncStream: () => {},
      createContinuousSyncStream: () => {},
    };

    this.sinon.stub(options, 'createHistoricalSyncStream')
      .callsFake(async () => {
        const stream = new BlockHeadersWithChainLocksStreamMock();
        this.sinon.spy(stream, 'on');
        this.sinon.spy(stream, 'destroy');
        this.sinon.spy(stream, 'removeListener');
        historicalStreams.push(stream);
        return stream;
      });

    this.sinon.stub(options, 'createContinuousSyncStream')
      .callsFake(async () => {
        const stream = new BlockHeadersWithChainLocksStreamMock();
        this.sinon.spy(stream, 'on');
        this.sinon.spy(stream, 'destroy');
        this.sinon.spy(stream, 'removeListener');
        continuousSyncStream = stream;
        return stream;
      });

    blockHeadersReader = new BlockHeadersReader(options);
    this.sinon.spy(blockHeadersReader, 'emit');
  });

  describe('#subscribeToHistoricalBatch', () => {
    let headers;
    let subscribeToHistoricalBatch;

    beforeEach(() => {
      headers = getHeadersFixture();

      subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatch(
        options.maxRetries,
      );
    });

    it('[data] should subscribe to block headers stream and hook on events', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      expect(blockHeadersReader.createHistoricalSyncStream).to.have.been.calledOnce;

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

    it('[error] should destroy stream in case headers batch was rejected', async () => {
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

      expect(stream.destroy).to.have.been.calledWith(rejectWith);
    });

    it('[error] should handle stream cancellation', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];
      stream.emit('error', {
        code: GrpcErrorCodes.CANCELLED,
      });

      await sleepOneTick();

      expect(blockHeadersReader.emit).to.have.been.calledWith(
        BlockHeadersReader.COMMANDS.HANDLE_STREAM_CANCELLATION, stream,
      );

      // Make sure we are not resubscribing to stream
      expect(blockHeadersReader.createHistoricalSyncStream)
        .to.have.been.calledOnce();

      // Make sure we are not emitting command to handle stream error
      expect(blockHeadersReader.emit)
        .to.have.not.been.calledWith(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR);
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

      const { firstCall, secondCall, thirdCall } = blockHeadersReader.emit;

      // Ensure first batch was emitted
      expect(firstCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(firstCall.args[1]).to.deep.equal({
        headers: firstBatch,
        headHeight: 1,
      });

      // Get new stream that has been created after error
      const newStream = historicalStreams[historicalStreams.length - 1];
      // Ensure that HANDLE_STREAM_RETRY command has been emitted
      expect(secondCall.args).to.deep.equal([
        BlockHeadersReader.COMMANDS.HANDLE_STREAM_RETRY,
        originalStream,
        newStream,
      ]);

      // Ensure that retry logic fetches only headers that weren't processed yet
      const fromBlock = startFrom + firstBatch.length;
      const count = headers.length - firstBatch.length;
      expect(blockHeadersReader.createHistoricalSyncStream.secondCall.args)
        .to.deep.equal([fromBlock, count]);

      // Ensure that second batch was emitted
      expect(thirdCall.args[0]).to.equal(BlockHeadersReader.EVENTS.BLOCK_HEADERS);
      expect(thirdCall.args[1]).to.deep.equal({
        headers: secondBatch,
        headHeight: startFrom + firstBatch.length,
      });
    });

    it('[error] should emit error in case of resubscribe failure', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];

      const resubscribeError = new Error('Error subscribing to block headers');

      // Prepare subscribe function to throw an error on second call
      blockHeadersReader.createHistoricalSyncStream.throws(resubscribeError);

      // Emit stream error to trigger retry attempt
      stream.emit('error', new Error('Invalid block headers'));

      await sleepOneTick();

      expect(blockHeadersReader.emit).to.have.been.calledOnce();

      expect(blockHeadersReader.emit).to.have.been.calledWith(
        BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR,
        stream,
        resubscribeError,
      );
    });

    it('[error] should emit error in case retry attempts were exhausted', async () => {
      await subscribeToHistoricalBatch(1, headers.length);

      const stream = historicalStreams[0];

      const firstError = new Error('firstError');
      stream.emit('error', firstError);

      await sleepOneTick();

      const secondError = new Error('secondError');
      stream.emit('error', secondError);

      const { firstCall, secondCall } = blockHeadersReader.emit;

      const newStream = historicalStreams[historicalStreams.length - 1];
      expect(firstCall.args)
        .to.deep.equal([
          BlockHeadersReader.COMMANDS.HANDLE_STREAM_RETRY,
          stream,
          newStream,
        ]);

      expect(secondCall.args)
        .to.deep.equal([
          BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR,
          stream,
          secondError,
        ]);
    });

    it('[end] should handle end event', async () => {
      await subscribeToHistoricalBatch(1, headers.length);
      const stream = historicalStreams[0];

      stream.emit('end');

      expect(blockHeadersReader.emit).to.have.been.calledOnceWithExactly(
        BlockHeadersReader.COMMANDS.HANDLE_STREAM_END,
        stream,
      );
    });
  });

  describe('#subscribeToNew', () => {
    let stream;
    beforeEach(async () => {
      stream = await blockHeadersReader.subscribeToNew(1);
    });
    //
    it('should subscribe to a stream', () => {
      // expect(blockHeadersReader.coreMethods.subscribeToBlockHeadersWithChainLocks)
      //   .to.be.calledOnce();
    });
    //
    // it('should hook on stream events', () => {
    //   expect(stream.on).to.be.calledWith('data');
    //   expect(stream.on).to.be.calledWith('error');
    // });
  });

  describe.skip('#readHistorical', () => {
    beforeEach(function () {
      this.sinon.spy(blockHeadersReader, 'subscribeToHistoricalBatch');
    });

    it('should create only one stream in case the amount of blocks is too small', async () => {
      await blockHeadersReader.readHistorical(1, Math.ceil(options.targetBatchSize * 1.4));
      expect(blockHeadersReader.subscribeToHistoricalBatch).to.be.calledOnce();
    });

    it('should evenly spread the load between streams', async () => {
      const fromBlock = 1;
      const toBlock = Math.round(options.targetBatchSize * 3.5 - 1);
      const totalAmount = toBlock - fromBlock + 1;
      const numStreams = Math.round(totalAmount / options.targetBatchSize);

      const itemsPerBatch = Math.ceil(totalAmount / numStreams);

      await blockHeadersReader.readHistorical(fromBlock, toBlock);

      const subscribeFunction = coreApiMock.subscribeToBlockHeadersWithChainLocks;
      expect(subscribeFunction).to.be.calledThrice();
      expect(subscribeFunction.getCall(0).args[0])
        .to.deep.equal({ fromBlockHeight: fromBlock, count: itemsPerBatch });
      expect(subscribeFunction.getCall(1).args[0])
        .to.deep.equal({ fromBlockHeight: fromBlock + itemsPerBatch, count: itemsPerBatch });
      expect(subscribeFunction.getCall(2).args[0])
        .to.deep.equal({
          fromBlockHeight: fromBlock + 2 * itemsPerBatch,
          count: totalAmount - itemsPerBatch * 2,
        });
    });

    it('should limit amount of streams in case batch size is too small compared to total amount', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 10);
      expect(blockHeadersReader.subscribeToHistoricalBatch.callCount)
        .to.equal(options.maxParallelStreams);
    });

    it('should throw an error in case the total amount of headers is less than 1', async () => {
      const from = 2;
      const to = 2;
      try {
        await blockHeadersReader.readHistorical(from, to);
      } catch (e) {
        expect(e.message).to.equal(`Invalid total amount of headers to read: ${to - from}`);
      }
    });

    it('should replace stream in historicalStreams in case of retry attempt', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 5);

      let streamToReplaceWith;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_RETRY, (_, newStream) => {
        streamToReplaceWith = newStream;
      });

      const streamToBreak = blockHeadersReader.historicalStreams[0];
      streamToBreak.emit('error', new Error('retry'));

      await sleepOneTick();

      expect(blockHeadersReader.historicalStreams[0]).to.equal(streamToReplaceWith);
    });

    it('should remove stream from historicalStreams array in case of end', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);
      const streamsAmount = blockHeadersReader.historicalStreams.length;

      const streamToFinish = blockHeadersReader.historicalStreams[0];
      streamToFinish.emit('end');

      expect(blockHeadersReader.historicalStreams.length).to.equal(streamsAmount - 1);
      expect(blockHeadersReader.historicalStreams.includes(streamToFinish)).to.be.false();
    });

    it('should remove stream from historicalStreams array in case of error', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 2);
      const streamsAmount = blockHeadersReader.historicalStreams.length;

      let streamToRemove;
      blockHeadersReader.on(BlockHeadersReader.COMMANDS.HANDLE_STREAM_ERROR, (_, stream) => {
        streamToRemove = stream;
      });

      let emittedError;
      blockHeadersReader.on(BlockHeadersReader.EVENTS.ERROR, (e) => {
        emittedError = e;
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

      expect(emittedError).to.deep.equal(lastError);
      expect(streamToRemove).to.exist();
      expect(blockHeadersReader.historicalStreams.length).to.equal(streamsAmount - 1);
      expect(blockHeadersReader.historicalStreams.includes(streamToRemove)).to.be.false();
    });
  });
});
