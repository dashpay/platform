const { expect } = require('chai');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');
const pow = require('@dashevo/dash-spv/lib/pow');
const utils = require('@dashevo/dash-spv/lib/utils');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');
const ReconnectableStream = require('../../../lib/transport/ReconnectableStream');
const BlockHeadersWithChainLocksStreamMock = require('../../../lib/test/mocks/BlockHeadersWithChainLocksStreamMock');

// Mines a regtest header on `prev`; `salt` makes sibling headers differ
const mine = (prev, salt = 0) => {
  const merkleRoot = Buffer.alloc(32);
  merkleRoot.writeUInt32LE(salt);
  let nonce = 0;
  let header;
  do {
    header = new BlockHeader({
      version: 2,
      prevHash: Buffer.from(prev.hash, 'hex').reverse(),
      merkleRoot,
      time: prev.time + 1,
      bits: 0x207fffff,
      nonce,
    });
    nonce += 1;
  } while (!utils.validProofOfWork(header));
  return header;
};

describe('BlockHeadersProvider - continuous sync after a reorg', () => {
  let blockHeadersProvider;
  let main;
  let updates;

  beforeEach(async () => {
    await SpvChain.wasmX11Ready();
    blockHeadersProvider = new BlockHeadersProvider({ network: 'regtest' });
    await blockHeadersProvider.initializeChainWith([], 0);

    main = [];
    let prev = blockHeadersProvider.spvChain.genesis;
    for (let i = 0; i < 5; i += 1) {
      prev = mine(prev);
      main.push(prev);
    }
    blockHeadersProvider.spvChain.addHeaders(main);

    blockHeadersProvider.state = BlockHeadersProvider.STATES.CONTINUOUS_SYNC;
    updates = [];
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, (headers, height) => {
      updates.push({ hashes: headers.map((h) => h.hash), height });
    });
  });

  it('should ask again from below the tip when only the new tip arrives, then reorg', () => {
    // The network switched from height 5 to a fork of heights 5 and 6, and
    // DAPI streamed only the new tip
    const forkAt5 = mine(main[3], 1);
    const forkAt6 = mine(forkAt5, 1);

    let rejection;
    blockHeadersProvider.headersHandler({ headers: [forkAt6], headHeight: 6 }, (e) => {
      rejection = e;
    });

    expect(rejection).to.be.an.instanceOf(SPVError);
    expect(blockHeadersProvider.spvChain.getOrphanChunks()).to.have.lengthOf(0);
    expect(blockHeadersProvider.getContinuousResumeHeight()).to.equal(4);

    // The stream reconnects from height 4 and DAPI sends the chain from there
    blockHeadersProvider.headersHandler(
      { headers: [main[3], forkAt5, forkAt6], headHeight: 4 },
      (e) => { throw e; },
    );

    expect(blockHeadersProvider.spvChain.getTipHash()).to.equal(forkAt6.hash);
    expect(updates).to.deep.equal([{ hashes: [forkAt5.hash, forkAt6.hash], height: 5 }]);
    expect(blockHeadersProvider.getContinuousResumeHeight()).to.equal(6);
  });

  it('should look further back each time headers still don\'t connect', () => {
    const forkAt4 = mine(main[2], 1);
    const forkAt5 = mine(forkAt4, 1);
    const forkAt6 = mine(forkAt5, 1);

    blockHeadersProvider.headersHandler({ headers: [forkAt6], headHeight: 6 }, () => {});
    expect(blockHeadersProvider.getContinuousResumeHeight()).to.equal(4);

    blockHeadersProvider.headersHandler({ headers: [forkAt5, forkAt6], headHeight: 4 }, () => {});
    expect(blockHeadersProvider.getContinuousResumeHeight()).to.equal(2);

    blockHeadersProvider.headersHandler(
      { headers: [main[1], main[2], forkAt4, forkAt5, forkAt6], headHeight: 2 },
      (e) => { throw e; },
    );
    expect(blockHeadersProvider.spvChain.getTipHash()).to.equal(forkAt6.hash);
    expect(updates[0].height).to.equal(4);
  });

  it('should report once when headers still don\'t connect at the deepest lookback', () => {
    const errors = [];
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => errors.push(e));
    blockHeadersProvider.continuousLookback = blockHeadersProvider.spvChain.confirmsBeforeFinal + 1;
    const unrelated = mine(mine(blockHeadersProvider.spvChain.genesis, 9), 9);

    blockHeadersProvider.headersHandler({ headers: [unrelated], headHeight: 6 }, () => {});
    blockHeadersProvider.headersHandler({ headers: [unrelated], headHeight: 6 }, () => {});

    expect(errors).to.have.lengthOf(1);
    expect(errors[0]).to.be.an.instanceOf(SPVError);
    expect(blockHeadersProvider.getContinuousResumeHeight()).to.equal(1);
  });

  it('should retry exactly once, even when nothing listens for errors', () => {
    blockHeadersProvider.continuousLookback = blockHeadersProvider.spvChain.confirmsBeforeFinal + 1;
    const unrelated = mine(mine(blockHeadersProvider.spvChain.genesis, 9), 9);
    let rejections = 0;

    blockHeadersProvider.headersHandler(
      { headers: [unrelated], headHeight: 6 },
      () => { rejections += 1; },
    );

    expect(rejections).to.equal(1);
  });

  it('should report once when the deepest lookback reaches a reorg off confirmed headers', () => {
    const errors = [];
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => errors.push(e));
    const { spvChain } = blockHeadersProvider;
    spvChain.confirmsBeforeFinal = 2;
    spvChain.checkPruneBlocks(); // heights 1 to 3 confirmed
    blockHeadersProvider.continuousLookback = 3;
    const offConfirmed = [mine(main[1], 1)]; // forks off height 2
    const rejected = [];

    blockHeadersProvider.headersHandler({ headers: offConfirmed, headHeight: 3 }, (e) => rejected.push(e));
    blockHeadersProvider.headersHandler({ headers: offConfirmed, headHeight: 3 }, (e) => rejected.push(e));

    expect(rejected).to.have.lengthOf(2);
    expect(rejected[0].message).to.include('forks off confirmed header');
    expect(errors).to.have.lengthOf(1);
    expect(errors[0].message).to.include('forks off confirmed header');
  });

  describe('through the stream transport', () => {
    const { getChainWork } = pow;
    let requestedHeights;
    let streams;
    let serverChain;
    let serverOpen;

    const notFound = (height) => {
      const error = new Error(`Block ${height} not found`);
      error.code = GrpcErrorCodes.NOT_FOUND;
      return error;
    };

    // Starts continuous sync over a real ReconnectableStream, from a DAPI
    // that has `serverChain`. The first stream opened before the reorg.
    const startSync = async (sinon) => {
      const subscribe = ({ fromBlockHeight }) => {
        requestedHeights.push(fromBlockHeight);
        const stream = new BlockHeadersWithChainLocksStreamMock(sinon);
        streams.push(stream);
        if (streams.length > 1) {
          setTimeout(() => {
            if (!serverOpen) {
              return;
            }
            if (fromBlockHeight > serverChain.length) {
              stream.emit('error', notFound(fromBlockHeight));
            } else {
              stream.sendHeaders(serverChain.slice(fromBlockHeight - 1));
            }
          }, 0);
        }
        return stream;
      };
      blockHeadersProvider.createContinuousSyncStream = (fromBlockHeight) => ReconnectableStream
        .create(subscribe, { maxRetriesOnError: -1, retryOnErrorDelay: 0, autoReconnectInterval: 0 })({
          fromBlockHeight,
        });
      blockHeadersProvider.state = BlockHeadersProvider.STATES.IDLE;
      await blockHeadersProvider.startContinuousSync(6);
    };

    // Resolves once `condition` holds, or fails with the heights sync asked for
    const waitFor = (condition) => new Promise((resolve, reject) => {
      const started = Date.now();
      const check = () => {
        if (condition()) {
          resolve();
        } else if (Date.now() - started > 300) {
          reject(new Error(`Sync stalled, requesting heights ${requestedHeights.slice(0, 8)}…`));
        } else {
          setTimeout(check, 5);
        }
      };
      check();
    });

    beforeEach(() => {
      requestedHeights = [];
      streams = [];
      serverOpen = true;
    });

    afterEach(async () => {
      // Ends the retries of a test that failed
      serverOpen = false;
      await blockHeadersProvider.stop();
      pow.getChainWork = getChainWork;
    });

    it('should look further back when DAPI has no block at the height asked for', async function it() {
      // The network switched from height 5 to a shorter fork with more work,
      // ending at height 3, and DAPI streamed only its tip
      const forkAt2 = mine(main[0], 1);
      const forkAt3 = mine(forkAt2, 1);
      serverChain = [main[0], forkAt2, forkAt3];
      const forkHashes = new Set([forkAt2.hash, forkAt3.hash]);
      pow.getChainWork = (headers) => (headers.some((h) => forkHashes.has(h.hash))
        ? 10n ** 30n : getChainWork(headers));

      await startSync(this.sinon);
      streams[0].sendHeaders([forkAt3]);
      await waitFor(() => blockHeadersProvider.spvChain.getTipHash() === forkAt3.hash);

      // 4 is above DAPI's new tip; only the NOT_FOUND it gets moves the request below
      expect(requestedHeights).to.deep.equal([6, 4, 2]);
      expect(updates).to.deep.equal([{ hashes: [forkAt2.hash, forkAt3.hash], height: 2 }]);
    });

    it('should keep retrying when an error listener throws on the report', async function it() {
      if (typeof window !== 'undefined') {
        // Catching what the listener throws needs Node's uncaughtException
        this.skip();
      }

      // A DAPI that has none of the heights asked for; with a lookback of at
      // most 3, sync reports on the 3rd NOT_FOUND
      serverChain = [];
      blockHeadersProvider.spvChain.confirmsBeforeFinal = 2;
      let reportedAt = null;
      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, () => {
        reportedAt = requestedHeights.length;
        throw new Error('listener failed');
      });

      const thrown = [];
      const mochaHandlers = process.listeners('uncaughtException');
      process.removeAllListeners('uncaughtException');
      process.on('uncaughtException', (e) => thrown.push(e));
      try {
        await startSync(this.sinon);
        streams[0].emit('error', notFound(6));
        await waitFor(() => reportedAt !== null && requestedHeights.length > reportedAt + 2);
      } finally {
        process.removeAllListeners('uncaughtException');
        mochaHandlers.forEach((handler) => process.on('uncaughtException', handler));
      }

      expect(thrown.map((e) => e.message)).to.deep.equal(['listener failed']);
      expect(requestedHeights.slice(0, 4)).to.deep.equal([6, 4, 3, 3]);
    });

    it('should report once when DAPI has no block even at the deepest lookback', async function it() {
      const errors = [];
      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => errors.push(e));
      const cancelled = new Error('Cancelled');
      cancelled.code = GrpcErrorCodes.CANCELLED;

      blockHeadersProvider.createContinuousSyncStream = async () => (
        new BlockHeadersWithChainLocksStreamMock(this.sinon)
      );
      blockHeadersProvider.state = BlockHeadersProvider.STATES.IDLE;
      await blockHeadersProvider.startContinuousSync(6);

      // A failure other than NOT_FOUND resumes where sync left off
      expect(blockHeadersProvider.getContinuousRetryHeight(cancelled)).to.equal(5);
      const resumeHeights = [];
      for (let i = 0; i < 10; i += 1) {
        resumeHeights.push(blockHeadersProvider.getContinuousRetryHeight(notFound(1)));
      }
      // The report waits for the stream to schedule its retry
      await new Promise((resolve) => { setTimeout(resolve, 10); });

      expect(resumeHeights.slice(0, 3)).to.deep.equal([4, 2, 1]);
      expect(resumeHeights[9]).to.equal(1);
      expect(errors).to.have.lengthOf(1);
      expect(errors[0].message).to.include('DAPI has no block');
    });
  });
});
