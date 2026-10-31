const { expect } = require('chai');
const { BlockHeader } = require('@dashevo/dashcore-lib');

const SpvChain = require('../lib/spvchain');
const pow = require('../lib/pow');
const utils = require('../lib/utils');

const REGTEST_BITS = 0x207fffff;

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
      bits: REGTEST_BITS,
      nonce,
    });
    nonce += 1;
  } while (!utils.validProofOfWork(header));
  return header;
};

const mineChain = (from, length, salt = 0) => {
  const headers = [];
  let prev = from;
  for (let i = 0; i < length; i += 1) {
    prev = mine(prev, salt);
    headers.push(prev);
  }
  return headers;
};

describe('SpvChain reorgs', () => {
  let chain;
  let main;
  let connected;

  before(async () => {
    await SpvChain.wasmX11Ready();
  });

  beforeEach(() => {
    chain = new SpvChain('regtest');
    chain.initialize();
    main = mineChain(chain.genesis, 5);
    chain.addHeaders(main);

    connected = [];
    chain.on(SpvChain.EVENTS.CONNECTED, (headers, firstHeight) => {
      connected.push({ hashes: headers.map((h) => h.hash), firstHeight });
    });
  });

  it('should switch to a fork from below the tip once it has more work', () => {
    const fork = mineChain(main[3], 2, 1); // heights 5 and 6, on height 4

    chain.addHeaders([fork[0]]);
    expect(chain.getTipHash()).to.equal(main[4].hash);

    chain.addHeaders([fork[1]]);

    expect(chain.getTipHash()).to.equal(fork[1].hash);
    expect(chain.getTipHeight()).to.equal(6);
    expect(chain.hashesByHeight.get(5)).to.equal(fork[0].hash);
    expect(chain.heightByHash.has(main[4].hash)).to.equal(false);
    expect(chain.getOrphanChunks()).to.have.lengthOf(0);
    expect(() => chain.validate()).to.not.throw();
    expect(connected).to.deep.equal([{ hashes: fork.map((h) => h.hash), firstHeight: 5 }]);
  });

  it('should keep following the new chain after a reorg', () => {
    const fork = mineChain(main[3], 2, 1);
    chain.addHeaders(fork);

    const next = mine(fork[1]);
    chain.addHeaders([next]);

    expect(chain.getTipHash()).to.equal(next.hash);
    expect(chain.getTipHeight()).to.equal(7);
  });

  it('should switch back when the old chain outgrows the fork', () => {
    const fork = mineChain(main[3], 2, 1);
    chain.addHeaders(fork);

    const oldChainExtension = mineChain(main[4], 2); // heights 6 and 7 on the first chain
    chain.addHeaders(oldChainExtension);

    expect(chain.getTipHash()).to.equal(oldChainExtension[1].hash);
    expect(chain.hashesByHeight.get(5)).to.equal(main[4].hash);
    expect(connected[connected.length - 1]).to.deep.equal({
      hashes: [main[4], ...oldChainExtension].map((h) => h.hash),
      firstHeight: 5,
    });
  });

  it('should report headers connected to the tip with their height', () => {
    const next = mineChain(main[4], 2);
    chain.addHeaders(next);

    expect(connected).to.deep.equal([{ hashes: next.map((h) => h.hash), firstHeight: 6 }]);
  });

  it('should reject a fork from below the confirmed headers', () => {
    chain = new SpvChain('regtest', 3);
    chain.initialize();
    chain.addHeaders(main);

    expect(() => chain.addHeaders(mineChain(main[0], 6, 1))).to.throw();
    expect(chain.getTipHash()).to.equal(main[4].hash);
  });
});

describe('SpvChain memory', () => {
  let chain;
  let headers;

  before(async () => {
    await SpvChain.wasmX11Ready();
    chain = new SpvChain('regtest');
    chain.initialize();
    headers = mineChain(chain.genesis, pow.MAX_HISTORY + 1000);
    chain.addHeaders(headers);
  });

  it('should keep only the confirmed headers validation reads', () => {
    expect(chain.getLongestChain()).to.have.lengthOf(100);
    expect(chain.prunedHeaders).to.have.lengthOf(pow.MAX_HISTORY);
    expect(chain.hashesByHeight.size).to.equal(pow.MAX_HISTORY + 100);
    expect(chain.heightByHash.size).to.equal(pow.MAX_HISTORY + 100);
    expect(() => chain.validate()).to.not.throw();
  });

  it('should still know the tip height', () => {
    expect(chain.getTipHeight()).to.equal(headers.length);
    expect(chain.hashesByHeight.get(headers.length)).to.equal(headers[headers.length - 1].hash);
  });

  it('should find kept headers by hash, confirmed or not', () => {
    const confirmed = headers[headers.length - 200];
    const unconfirmed = headers[headers.length - 50];

    expect(chain.getHeader(confirmed.hash).hash).to.equal(confirmed.hash);
    expect(chain.getHeader(unconfirmed.hash).hash).to.equal(unconfirmed.hash);
    expect(chain.getHeader(headers[0].hash)).to.equal(undefined);
  });

  it('should keep validating new headers', () => {
    const next = mineChain(headers[headers.length - 1], 3);
    chain.addHeaders(next);

    expect(chain.getTipHash()).to.equal(next[2].hash);
    expect(chain.prunedHeaders).to.have.lengthOf(pow.MAX_HISTORY);
  });
});

describe('SpvChain forks', () => {
  let chain;
  let main;

  before(async () => {
    await SpvChain.wasmX11Ready();
  });

  beforeEach(() => {
    chain = new SpvChain('regtest', 3);
    chain.initialize();
    main = mineChain(chain.genesis, 5);
    chain.addHeaders(main);
  });

  it('should keep the main chain first while a fork has less work', () => {
    const fork = mine(main[3], 1);
    chain.addHeaders([fork]);

    expect(chain.getAllBranches()).to.have.lengthOf(2);
    expect(chain.getLongestChain()[2].hash).to.equal(main[4].hash);
    expect(chain.getTipHash()).to.equal(main[4].hash);
  });

  it('should ignore fork headers it already has', () => {
    const fork = mineChain(main[3], 2, 1);
    chain.addHeaders(fork);

    const branches = chain.getAllBranches().map((branch) => branch.map((h) => h.hash));

    chain.addHeaders([main[3], ...fork]);

    expect(chain.getAllBranches().map((branch) => branch.map((h) => h.hash)))
      .to.deep.equal(branches);
    expect(chain.getTipHash()).to.equal(fork[1].hash);
  });

  it('should start a fork off a fork header', () => {
    // Same work as the main chain's heights 4 and 5, so no reorg
    const fork = mineChain(main[2], 2, 1);
    chain.addHeaders(fork);
    const forkOfFork = mineChain(fork[0], 2, 2);

    chain.addHeaders(forkOfFork);

    // Heights 4' 5'' 6'' have more work than 4 and 5
    expect(chain.getTipHash()).to.equal(forkOfFork[1].hash);
    expect(chain.hashesByHeight.get(4)).to.equal(fork[0].hash);
    expect(chain.getTipHeight()).to.equal(6);
    expect(() => chain.validate()).to.not.throw();
    // The two replaced headers and the other branch of the first fork remain
    expect(chain.getAllBranches().slice(1).map((branch) => branch.map((h) => h.hash)))
      .to.have.deep.members([[main[3].hash, main[4].hash], [fork[0].hash, fork[1].hash]]);
  });

  it('should keep at most 10 forks, dropping the oldest', () => {
    chain = new SpvChain('regtest', 100);
    chain.initialize();
    chain.addHeaders(main);
    const forks = [];
    for (let salt = 1; salt <= 11; salt += 1) {
      forks.push(mine(main[3], salt));
      chain.addHeaders([forks[forks.length - 1]]);
    }

    expect(chain.getAllBranches()).to.have.lengthOf(11);
    expect(chain.hasHeader(forks[0].hash)).to.equal(false);
    expect(chain.hasHeader(forks[10].hash)).to.equal(true);
  });

  it('should reorg to a fork much longer than the unconfirmed headers when it has more work', () => {
    // As after a wallet slept through many blocks while its tip was replaced
    const fork = mineChain(main[3], 20, 1);

    chain.addHeaders(fork);

    expect(chain.getTipHash()).to.equal(fork[19].hash);
    expect(chain.getTipHeight()).to.equal(24);
    expect(() => chain.validate()).to.not.throw();
  });

  it('should not keep a fork longer than twice the unconfirmed headers without more work', () => {
    // Every regtest header has the same work, so make the fork's count for less
    const { getChainWork } = pow;
    const fork = mineChain(main[2], 7, 1);
    const forkHashes = new Set(fork.map((h) => h.hash));
    pow.getChainWork = (headers) => (headers.some((h) => forkHashes.has(h.hash))
      ? 0n : getChainWork(headers));
    try {
      expect(() => chain.addHeaders(fork)).to.throw('longer than 6 headers without more work');
    } finally {
      pow.getChainWork = getChainWork;
    }
    expect(chain.getAllBranches()).to.have.lengthOf(1);
    expect(chain.getTipHash()).to.equal(main[4].hash);
  });

  it('should not keep a fork whose first header is invalid', () => {
    const { hash, ...fields } = mine(main[3], 1).toObject();
    const invalid = new BlockHeader({ ...fields, bits: 0x1d00ffff });

    expect(() => chain.addHeaders([invalid])).to.throw();
    expect(chain.getAllBranches()).to.have.lengthOf(1);
  });

  it('should drop a fork once it could only replace confirmed headers', () => {
    chain.addHeaders([mine(main[3], 1)]); // forks off height 4

    // Heights 5 to 7 unconfirmed: the fork could still replace them
    chain.addHeaders(mineChain(main[4], 2));
    expect(chain.getAllBranches()).to.have.lengthOf(2);

    // Height 5 confirmed
    chain.addHeaders([mine(chain.getTipHeader())]);
    expect(chain.getAllBranches()).to.have.lengthOf(1);
  });
});
