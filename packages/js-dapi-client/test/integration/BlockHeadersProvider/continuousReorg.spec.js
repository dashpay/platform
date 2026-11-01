const { expect } = require('chai');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const { SpvChain, SPVError } = require('@dashevo/dash-spv');
const utils = require('@dashevo/dash-spv/lib/utils');

const BlockHeadersProvider = require('../../../lib/BlockHeadersProvider/BlockHeadersProvider');

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

  it('should retry before reporting, even when nothing listens for errors', () => {
    blockHeadersProvider.continuousLookback = blockHeadersProvider.spvChain.confirmsBeforeFinal + 1;
    const unrelated = mine(mine(blockHeadersProvider.spvChain.genesis, 9), 9);
    let rejected = false;

    blockHeadersProvider.headersHandler(
      { headers: [unrelated], headHeight: 6 },
      () => { rejected = true; },
    );

    expect(rejected).to.equal(true);
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
});
