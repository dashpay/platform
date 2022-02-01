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

  let coreApiMock;
  let blockHeadersReader;
  let streamMock;
  beforeEach(function () {
    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: () => {},
    };

    this.sinon.stub(coreApiMock, 'subscribeToBlockHeadersWithChainLocks').callsFake(async () => {
      streamMock = new EventEmitter();
      streamMock.destroy = (e) => {
        streamMock.emit('error', e);
      };
      this.sinon.spy(streamMock, 'on');

      return streamMock;
    });

    options = {
      coreMethods: coreApiMock,
      maxRetries: 5,
      maxParallelStreams: 6,
      targetBatchSize: 10,
    };

    blockHeadersReader = new BlockHeadersReader(options);
  });

  describe('#subscribeToHistoricalBatch', () => {
    const numHeaders = 20;

    beforeEach(async () => {
      const subscribeToHistoricalBatch = blockHeadersReader.subscribeToHistoricalBatchHOF(
        options.maxRetries,
      );
      await subscribeToHistoricalBatch(1, numHeaders);
    });

    it('should subscribe to a stream', () => {
      expect(blockHeadersReader.coreMethods.subscribeToBlockHeadersWithChainLocks)
        .to.be.calledOnce();
    });

    it('should hook on stream events', () => {
      expect(streamMock.on).to.be.calledWith('data');
      expect(streamMock.on).to.be.calledWith('error');
      expect(streamMock.on).to.be.calledWith('end');
    });

    it('should only fetch remaining amount of headers in case of the retry attempt', async () => {
      const headers = Array.from({ length: numHeaders }).map((_, i) => `0x${i.toString(16)}`);
      const firstBatch = headers.slice(0, headers.length / 3);
      // Emit first batch of data
      streamMock.emit('data', {
        getBlockHeaders: () => ({
          getHeadersList: () => firstBatch,
        }),
      });

      streamMock.emit('error', new Error('retry'));

      await sleepOneTick();

      const subscribeStub = coreApiMock.subscribeToBlockHeadersWithChainLocks;
      expect(subscribeStub).to.be.calledTwice();
      expect(subscribeStub.firstCall.args[0]).to.deep.equal({
        fromBlockHeight: 1,
        count: headers.length,
      });
      expect(subscribeStub.secondCall.args[0]).to.deep.equal({
        fromBlockHeight: firstBatch.length + 1,
        count: headers.length - firstBatch.length,
      });
    });
  });

  describe('#subscribeToNew', () => {
    let stream;
    beforeEach(async () => {
      stream = await blockHeadersReader.subscribeToNew(1);
    });

    it('should subscribe to a stream', () => {
      expect(blockHeadersReader.coreMethods.subscribeToBlockHeadersWithChainLocks)
        .to.be.calledOnce();
    });

    it('should hook on stream events', () => {
      expect(stream.on).to.be.calledWith('data');
      expect(stream.on).to.be.calledWith('error');
    });
  });

  describe('#readHistorical', () => {
    beforeEach(function () {
      this.sinon.spy(blockHeadersReader, 'subscribeToHistoricalBatchHOF');
    });

    it('should create only one stream in case the amount of blocks is too small', async () => {
      await blockHeadersReader.readHistorical(1, Math.ceil(options.targetBatchSize * 1.4));
      expect(blockHeadersReader.subscribeToHistoricalBatchHOF).to.be.calledOnce();
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
      expect(blockHeadersReader.subscribeToHistoricalBatchHOF.callCount)
        .to.equal(options.maxParallelStreams);
    });

    it('should throw an error in case the total amount of headers is less than 1', async () => {
      const from = 2;
      const to = 1;
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

      // Exhaust all retry attempts to actually throw an error
      for (let i = 0; i < options.maxRetries + 1; i += 1) {
        const [streamToBreak] = blockHeadersReader.historicalStreams;
        streamToBreak.emit('error', new Error('retry'));
        // eslint-disable-next-line no-await-in-loop
        await sleepOneTick();
      }

      expect(streamToRemove).to.exist();
      expect(blockHeadersReader.historicalStreams.length).to.equal(streamsAmount - 1);
      expect(blockHeadersReader.historicalStreams.includes(streamToRemove)).to.be.false();
    });
  });
});
