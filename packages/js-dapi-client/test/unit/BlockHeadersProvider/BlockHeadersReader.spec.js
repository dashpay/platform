const BlockHeadersReader = require('../../../lib/BlockHeadersProvider/BlockHeadersReader');

describe('BlockHeadersReader', () => {
  let options;

  let coreApiMock;
  let blockHeadersReader;
  let streamMock;
  beforeEach(function () {
    streamMock = {
      on: this.sinon.stub(),
    };
    coreApiMock = {
      subscribeToBlockHeadersWithChainLocks: this.sinon.stub().resolves(streamMock),
    };

    options = {
      coreMethods: coreApiMock,
      maxRetries: 5,
      maxParallelStreams: 6,
      targetBatchSize: 10,
    };

    blockHeadersReader = new BlockHeadersReader(options);
  });

  describe('#subscribeToStream', () => {
    beforeEach(async () => {
      const subscribeToStream = blockHeadersReader.createSubscribeToStream(5);
      subscribeToStream(1, 1);
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
    let subscribeFunction;
    beforeEach(function () {
      subscribeFunction = this.sinon.stub();
      blockHeadersReader.createSubscribeToStream = this.sinon.stub().returns(subscribeFunction);
    });

    it('should create only one stream in case the amount of blocks is too small', async () => {
      await blockHeadersReader.readHistorical(1, Math.ceil(options.targetBatchSize * 1.4));
      expect(blockHeadersReader.createSubscribeToStream).to.be.calledOnce();
    });

    it('should evenly spread the load between streams', async () => {
      const fromBlock = 1;
      const toBlock = Math.round(options.targetBatchSize * 3.5 - 1);
      const totalAmount = toBlock - fromBlock + 1;
      const numStreams = Math.round(totalAmount / options.targetBatchSize);

      const itemsPerBatch = Math.ceil(totalAmount / numStreams);

      await blockHeadersReader.readHistorical(fromBlock, toBlock);

      expect(subscribeFunction).to.be.calledThrice();
      expect(subscribeFunction.getCall(0).args)
        .to.deep.equal([fromBlock, itemsPerBatch]);
      expect(subscribeFunction.getCall(1).args)
        .to.deep.equal([fromBlock + itemsPerBatch, itemsPerBatch]);
      expect(subscribeFunction.getCall(2).args)
        .to.deep.equal([fromBlock + 2 * itemsPerBatch, totalAmount - itemsPerBatch * 2]);
    });

    it('should limit amount of streams in case batch size is too small compared to total amount', async () => {
      await blockHeadersReader.readHistorical(1, options.targetBatchSize * 10);
      expect(blockHeadersReader.createSubscribeToStream.callCount)
        .to.equal(options.maxParallelStreams);
    });

    it('#should throw an error in case the total amount of headers is less than 1', async () => {
      const from = 2;
      const to = 1;
      try {
        await blockHeadersReader.readHistorical(from, to);
      } catch (e) {
        expect(e.message).to.equal(`Invalid total amount of headers to read: ${to - from}`);
      }
    });
  });
});
