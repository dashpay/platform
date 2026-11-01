const { expect } = require('chai');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const BlockHeadersWithChainLocksStreamMock = require('../../../lib/test/mocks/BlockHeadersWithChainLocksStreamMock');
const mockHeadersChain = require('../../../lib/test/mocks/mockHeadersChain');

describe('BlockHeadersProvider - integration', function describe() {
  this.timeout(6000);

  let blockHeadersProvider;
  let historicalStreams = [];
  let continuousStream;

  const createBlockHeadersProvider = async (sinon, opts = {}) => {
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

    blockHeadersProvider = new BlockHeadersProvider(
      opts,
      (fromBlockHeight, count) => subscribeToBlockHeadersWithChainLocks({
        fromBlockHeight,
        count,
      }),
      () => subscribeToBlockHeadersWithChainLocks({
        count: 0,
      }),
    );
    await blockHeadersProvider.initializeChainWith([], 0);
  };

  // Start from height bigger than the first block
  // because we need to make sure that spv chain could bootstrap itself
  // from any header
  const fromBlockHeight = 10;
  const numHeaders = 100;
  const newHeadersAmount = 2;
  const numStreams = 5;
  const historicalHeadersAmount = numHeaders - newHeadersAmount;

  // -1 because fromBlockHeight is inclusive
  const chainHeight = fromBlockHeight + historicalHeadersAmount - 1;
  const historicalBatchSize = Math.round(historicalHeadersAmount / numStreams);
  const numBatches = Math.ceil(historicalHeadersAmount / historicalBatchSize);

  let headers;

  before(async function () {
    const chainWithCheckpoint = await mockHeadersChain(
      'regtest',
      numHeaders + 1,
      undefined,
      { mine: true },
    );
    const trustedCheckpoint = chainWithCheckpoint[0];
    headers = chainWithCheckpoint.slice(1);

    await createBlockHeadersProvider(this.sinon, {
      network: 'regtest',
      targetBatchSize: historicalBatchSize,
    });
    blockHeadersProvider.spvChain.reset();
    await blockHeadersProvider.initializeChainWith(
      [trustedCheckpoint],
      fromBlockHeight - 1,
    );
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
        stream.cancel();
      }
    });

    const { spvChain } = blockHeadersProvider;

    // Headers added from the tail should be orphaned, and not reported as
    // part of the chain yet
    expect(spvChain.getOrphanChunks()).to.have.length(numBatches - 1);
    expect(spvChain.getLongestChain()).to.have.length(1);
    expect(blockHeadersProvider.emit)
      .to.not.have.been.calledWith(BlockHeadersProvider.EVENTS.CHAIN_UPDATED);
  });

  it('should read batch from the head and form longest chain', async () => {
    const headersToSend = headers.slice(0, historicalBatchSize);
    historicalStreams[0].sendHeaders(headersToSend);
    historicalStreams[0].end();

    const { spvChain } = blockHeadersProvider;
    expect(spvChain.getLongestChain({ withPruned: true }))
      .to.have.length(historicalHeadersAmount + 1);
    expect(blockHeadersProvider.emit).to
      .have.been.calledWith(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);

    // The head batch, then each orphaned batch it connected, at its height
    const updates = blockHeadersProvider.emit.getCalls()
      .filter(({ args }) => args[0] === BlockHeadersProvider.EVENTS.CHAIN_UPDATED);
    expect(updates).to.have.length(numBatches);
    updates.forEach(({ args }, i) => {
      const from = i * historicalBatchSize;
      const to = Math.min((i + 1) * historicalBatchSize, historicalHeadersAmount);

      expect(args[1].map((header) => header.toString()))
        .to.deep.equal(headers.slice(from, to).map((header) => header.toString()));
      expect(args[2]).to.equal(fromBlockHeight + from);
    });
  });

  it('should start continuous sync and add to existing chain', async () => {
    await blockHeadersProvider.startContinuousSync(chainHeight);

    // +1 because fromBlockHeight is inclusive
    const headersToSend = headers.slice(-(newHeadersAmount + 1));
    continuousStream.sendHeaders(headersToSend);

    const { spvChain } = blockHeadersProvider;

    expect(spvChain.getLongestChain({ withPruned: true }))
      .to.have.length(headers.length + 1);
    const { args } = blockHeadersProvider.emit.lastCall;

    const expectedHeaders = headersToSend.slice(1).map((header) => header.toString());
    const emittedHeaders = args[1].map((header) => header.toString());
    const headHeight = args[2];

    expect(emittedHeaders).to.deep.equal(expectedHeaders);
    expect(headHeight).to.equal(chainHeight + 1);
    await blockHeadersProvider.stop();
  });

  it('should accept the first historical batch after initializing from genesis', async function shouldAcceptFirstHistoricalBatch() {
    const chainFromGenesis = await mockHeadersChain(
      'regtest',
      2,
      undefined,
      { mine: true },
    );
    await createBlockHeadersProvider(this.sinon, {
      network: 'regtest',
      targetBatchSize: historicalBatchSize,
    });
    this.sinon.spy(blockHeadersProvider, 'emit');

    await blockHeadersProvider.readHistorical(1, 1);
    historicalStreams[0].sendHeaders(chainFromGenesis.slice(1));
    historicalStreams[0].end();

    expect(blockHeadersProvider.spvChain.getLongestChain({ withPruned: true }))
      .to.have.length(2);
    expect(blockHeadersProvider.emit)
      .to.have.been.calledWith(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
  });

  it('should not hold every batch as orphans when a server sends headers that don\'t connect', async function shouldBoundOrphans() {
    const chainFromGenesis = await mockHeadersChain('regtest', 62, undefined, { mine: true });
    await createBlockHeadersProvider(this.sinon, {
      network: 'regtest',
      targetBatchSize: 10,
      maxParallelStreams: 2,
    });
    const errors = [];
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => errors.push(e));
    const subscribedFrom = [];
    const createStream = blockHeadersProvider.createHistoricalSyncStream;
    blockHeadersProvider.createHistoricalSyncStream = (from, count) => {
      subscribedFrom.push(from);
      return createStream(from, count);
    };

    await blockHeadersProvider.readHistorical(1, 60);

    // Every batch arrives shifted forward by one header, so none connects
    for (let i = 0; i < historicalStreams.length; i += 1) {
      const from = subscribedFrom[i];
      historicalStreams[i].sendHeaders(chainFromGenesis.slice(from + 1, from + 11));
      historicalStreams[i].end();
      // eslint-disable-next-line no-await-in-loop
      await new Promise(setImmediate);
    }

    expect(errors.map((e) => e.message)).to.include('Headers from height 1 don\'t connect to the chain');
    expect(historicalStreams).to.have.lengthOf(2);
    const { spvChain } = blockHeadersProvider;
    expect(spvChain.getOrphanChunks().reduce((n, chunk) => n + chunk.length, 0)).to.be.at.most(20);
  });
});
