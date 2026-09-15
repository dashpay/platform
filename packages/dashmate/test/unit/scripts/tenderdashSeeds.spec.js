import { generateSnapshot, checkSnapshot } from '../../../scripts/tenderdashSeeds.js';
import seedSetHash from '../../../src/tenderdash/seedSetHash.js';

const NOW = 1800000000;
const HASH = 'a'.repeat(64);

function evonode(index) {
  return {
    type: 'Evo',
    state: {
      platformNodeID: index.toString(16).padStart(40, '0'),
      service: `8.1.1.${index}:9999`,
      platformP2PPort: 26656,
      PoSeBanHeight: -1,
    },
  };
}

describe('Tenderdash seed generation', () => {
  let info;
  let registry;
  let previous;
  let calls;

  function rpc(...args) {
    calls.push(args);
    if (args[0] === 'getblockchaininfo') { return info; }
    if (args[0] === 'protx') { return registry; }
    if (args[0] === 'getblockhash') { return HASH; }
    throw new Error('Unexpected RPC');
  }

  beforeEach(() => {
    info = {
      chain: 'main',
blocks: 100,
headers: 100,
initialblockdownload: false,
      bestblockhash: HASH,
time: NOW - 60,
    };
    registry = Array.from({ length: 30 }, (_, index) => evonode(index + 1));
    previous = { seeds: [{ id: 'f'.repeat(40), host: '8.9.1.1', port: 1234 }], previousSeedSetHashes: [] };
    calls = [];
  });

  it('should deterministically select unique registered evonodes at a pinned block', () => {
    const snapshot = generateSnapshot(rpc, 'mainnet', previous, NOW);
    expect(snapshot.seeds).to.have.length(20);
    expect(calls).to.deep.equal([
      ['getblockchaininfo'], ['protx', 'list', 'valid', 'true', '100'], ['getblockhash', '100'],
    ]);
    registry.reverse();
    expect(generateSnapshot(rpc, 'mainnet', previous, NOW)).to.deep.equal(snapshot);
    expect(snapshot).to.include({ chain: 'main', height: 100, blockHash: HASH, blockTime: NOW - 60 });
  });

  it('should use registered Platform endpoints instead of the Core service address', () => {
    registry[0].state.addresses = { platform_p2p: ['9.1.1.1:12345'] };
    const snapshot = generateSnapshot(rpc, 'mainnet', previous, NOW);
    expect(snapshot.seeds[0]).to.deep.equal({ id: '1'.padStart(40, '0'), host: '9.1.1.1', port: 12345 });
  });

  it('should exclude banned, non-Evo, invalid and duplicate endpoints', () => {
    registry[0].type = 'Regular';
    registry[1].state.PoSeBanHeight = 10;
    registry[2].metaInfo = { is_platform_banned: true };
    registry[3].state.platformNodeID = 'invalid';
    registry[4].state.platformP2PPort = 0;
    registry[5].state.service = 'example.org:9999';
    registry[6].state.platformNodeID = '0'.repeat(40);
    registry.push(registry[7], { ...registry[7], state: { ...registry[7].state, service: '9.2.2.2:9999' } });
    registry.push({ ...registry[8], state: { ...registry[8].state, platformNodeID: 'f'.repeat(40) } });
    const { seeds } = generateSnapshot(rpc, 'mainnet', previous, NOW);
    expect(seeds).to.have.length(20);
    expect(new Set(seeds.map(seed => seed.id)).size).to.equal(20);
    expect(new Set(seeds.map(seed => seed.host)).size).to.equal(20);
    expect(seeds.every(seed => !['8.1.1.1', '8.1.1.2', '8.1.1.3', '8.1.1.4', '8.1.1.5', '8.1.1.6', '8.1.1.7'].includes(seed.host))).to.equal(true);
  });

  it('should retain fingerprints across multiple refreshes', () => {
    const first = generateSnapshot(rpc, 'mainnet', previous, NOW);
    registry = registry.slice(5);
    const second = generateSnapshot(rpc, 'mainnet', first, NOW);
    expect(second.previousSeedSetHashes).to.include(seedSetHash(previous.seeds));
    expect(second.previousSeedSetHashes).to.include(seedSetHash(first.seeds));
    const third = generateSnapshot(rpc, 'mainnet', second, NOW);
    expect(new Set(third.previousSeedSetHashes).size).to.equal(third.previousSeedSetHashes.length);
  });

  [
    ['wrong network', { chain: 'test' }],
    ['initial block download', { initialblockdownload: true }],
    ['unsynced headers', { headers: 101 }],
    ['old chain tip', { time: NOW - 86401 }],
    ['missing tip time', { time: undefined }],
    ['future tip', { time: NOW + 7200 }],
  ].forEach(([name, change]) => {
    it(`should reject ${name}`, () => {
      Object.assign(info, change);
      expect(() => generateSnapshot(rpc, 'mainnet', previous, NOW)).to.throw();
    });
  });

  it('should reject an empty registry or too few valid evonodes', () => {
    [[], {}, registry.slice(0, 4)].forEach(entries => {
      registry = entries;
      expect(() => generateSnapshot(rpc, 'mainnet', previous, NOW)).to.throw();
    });
  });

  it('should reject a reorganization during generation', () => {
    info.bestblockhash = 'b'.repeat(64);
    expect(() => generateSnapshot(rpc, 'mainnet', previous, NOW)).to.throw('reorganized');
  });

  it('should validate testnet independently', () => {
    info.chain = 'test';
    registry = registry.map(node => ({ ...node, state: { ...node.state, platformP2PPort: 36656 } }));
    const snapshot = generateSnapshot(rpc, 'testnet', previous, NOW);
    expect(snapshot.chain).to.equal('test');
    expect(snapshot.seeds.every(seed => seed.port === 36656)).to.equal(true);
  });

  it('should reject stale snapshots at publishing time', () => {
    const snapshot = generateSnapshot(rpc, 'mainnet', previous, NOW);
    expect(() => checkSnapshot(snapshot, 'mainnet', NOW + 7 * 86400)).to.throw('stale');
    expect(() => checkSnapshot(snapshot, 'mainnet', NOW)).to.not.throw();
  });

  it('should reject malformed snapshots at publishing time', () => {
    const snapshot = generateSnapshot(rpc, 'mainnet', previous, NOW);
    for (const change of [
      { chain: 'test' }, { height: 0 }, { blockHash: 'invalid' }, { blockTime: NOW + 7200 },
      { seeds: [] }, { seeds: [...snapshot.seeds.slice(1), snapshot.seeds[1]] },
      { previousSeedSetHashes: ['invalid'] },
    ]) {
      expect(() => checkSnapshot({ ...snapshot, ...change }, 'mainnet', NOW)).to.throw();
    }
  });
});
