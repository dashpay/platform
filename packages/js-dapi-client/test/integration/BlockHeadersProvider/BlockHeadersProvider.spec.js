const { expect } = require('chai');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const BlockHeadersWithChainLocksStreamMock = require('../../../lib/test/mocks/BlockHeadersWithChainLocksStreamMock');
const mockHeadersChain = require('../../../lib/test/mocks/mockHeadersChain');

describe('BlockHeadersProvider - integration', () => {
  let blockHeadersProvider;
  let historicalStreams = [];
  let continuousStream;

  const createBlockHeadersProvider = (sinon, opts = {}) => {
    historicalStreams = [];
    continuousStream = null;

    const subscribeToBlockHeadersWithChainLocks = sinon.stub();

    subscribeToBlockHeadersWithChainLocks
      .callsFake(({ count }) => {
        const stream = new BlockHeadersWithChainLocksStreamMock(sinon);
        if (count > 0) {
          historicalStreams.push(stream);
        } else {
          continuousStream = stream;
        }

        return stream;
      });

    blockHeadersProvider = new BlockHeadersProvider(opts);

    blockHeadersProvider.setCoreMethods({
      subscribeToBlockHeadersWithChainLocks,
    });
  };

  // Start from height bigger than the first block
  // because we need to make sure that spv chain could bootstrap itself
  // from any header
  const fromBlockHeight = 10;
  const numHeaders = 500;
  const newHeadersAmount = 2;
  const numStreams = 5;
  const historicalHeadersAmount = numHeaders - newHeadersAmount;

  // -1 because fromBlockHeight is inclusive
  const chainHeight = fromBlockHeight + historicalHeadersAmount - 1;
  const historicalBatchSize = Math.round(historicalHeadersAmount / numStreams);

  let headers;

  before(async function () {
    headers = mockHeadersChain('testnet', numHeaders);

    createBlockHeadersProvider(this.sinon, {
      targetBatchSize: historicalBatchSize,
    });
  });

  beforeEach(function () {
    this.sinon.spy(blockHeadersProvider, 'emit');
  });

  it('should read first historical batches from the tail', async () => {
    await blockHeadersProvider.readHistorical(fromBlockHeight, chainHeight);

    historicalStreams.forEach((stream, i) => {
      if (i !== 0) {
        const from = i * historicalBatchSize;
        let to = (i + 1) * historicalBatchSize;
        to = to > historicalHeadersAmount ? historicalHeadersAmount : to;

        stream.sendHeaders(headers.slice(from, to));
        stream.destroy();
      }
    });

    const numBatches = headers.length / historicalBatchSize;

    const { spvChain } = blockHeadersProvider;

    // Headers added from the tail should be orphaned
    expect(spvChain.getOrphanChunks()).to.have.length(4);
    expect(spvChain.getLongestChain()).to.have.length(0);
    expect(blockHeadersProvider.emit.callCount).to.equal(4);
    expect(blockHeadersProvider.emit)
      .to.have.been.calledWith(BlockHeadersProvider.EVENTS.CHAIN_UPDATED);

    for (let i = 1; i < numBatches; i += 1) {
      const { args } = blockHeadersProvider.emit.getCall(i - 1);
      const emittedHeaders = args[1].map((header) => header.toString());

      const from = i * historicalBatchSize;
      let to = historicalBatchSize * (i + 1);
      to = to > historicalHeadersAmount ? historicalHeadersAmount : to;

      const expectedHeaders = headers
        .slice(from, to)
        .map((header) => header.toString());
      const batchHeadHeight = fromBlockHeight + i * historicalBatchSize;

      expect(emittedHeaders).to.deep.equal(expectedHeaders);
      expect(args[2]).to.equal(batchHeadHeight);
    }
  });

  it('should read batch from the head and form longest chain', async () => {
    const headersToSend = headers.slice(0, historicalBatchSize);
    historicalStreams[0].sendHeaders(headersToSend);
    historicalStreams[0].destroy();

    const { spvChain } = blockHeadersProvider;
    expect(spvChain.getLongestChain({ withPruned: true }))
      .to.have.length(historicalHeadersAmount);
    expect(blockHeadersProvider.emit).to
      .have.been.calledWith(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);

    const { args } = blockHeadersProvider.emit.getCall(0);
    const expectedHeaders = headersToSend.map((header) => header.toString());
    const emittedHeaders = args[1].map((header) => header.toString());
    const headHeight = args[2];

    expect(emittedHeaders).to.deep.equal(expectedHeaders);
    expect(headHeight).to.equal(fromBlockHeight);
  });

  it('should start continuous sync and add to existing chain', async () => {
    await blockHeadersProvider.startContinuousSync(chainHeight);

    // +1 because fromBlockHeight is inclusive
    const headersToSend = headers.slice(-(newHeadersAmount + 1));
    continuousStream.sendHeaders(headersToSend);

    const { spvChain } = blockHeadersProvider;

    expect(spvChain.getLongestChain({ withPruned: true }))
      .to.have.length(headers.length);
    const { args } = blockHeadersProvider.emit.lastCall;

    const expectedHeaders = headersToSend.slice(1).map((header) => header.toString());
    const emittedHeaders = args[1].map((header) => header.toString());
    const headHeight = args[2];

    expect(emittedHeaders).to.deep.equal(expectedHeaders);
    expect(headHeight).to.equal(chainHeight + 1);
  });
});
