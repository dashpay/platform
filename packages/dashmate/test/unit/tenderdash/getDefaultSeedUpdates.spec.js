import tenderdashSeeds from '../../../configs/defaults/tenderdashSeeds.js';
import getDefaultSeedUpdates from '../../../src/tenderdash/getDefaultSeedUpdates.js';
import seedSetHash from '../../../src/tenderdash/seedSetHash.js';

const oldSeeds = [
  { id: 'a'.repeat(40), host: '8.1.1.1', port: 26656 },
  { id: 'b'.repeat(40), host: '8.1.1.2', port: 26656 },
];
const config = (seeds, { network = 'mainnet', node } = {}) => ({
  network, platform: { drive: { tenderdash: { node, p2p: { seeds } } } },
});

describe('getDefaultSeedUpdates', () => {
  let history;
  beforeEach(() => {
    history = tenderdashSeeds.mainnet.previousSeedSetHashes;
    tenderdashSeeds.mainnet.previousSeedSetHashes = [...history, seedSetHash(oldSeeds)];
  });
  afterEach(() => { tenderdashSeeds.mainnet.previousSeedSetHashes = history; });

  it('should recognize reordered and duplicated defaults without mutating the input', () => {
    const configs = { example: config([oldSeeds[1], oldSeeds[0], oldSeeds[1]]) };
    const original = structuredClone(configs);
    const updates = getDefaultSeedUpdates(configs);
    expect(updates).to.deep.equal([['example', tenderdashSeeds.mainnet.seeds]]);
    expect(configs).to.deep.equal(original);
    expect(updates[0][1]).not.to.equal(tenderdashSeeds.mainnet.seeds);
  });

  it('should remove self on refresh and repair an already installed stock list', () => {
    const defaults = tenderdashSeeds.mainnet;
    const originalSeeds = defaults.seeds;
    const [self, other] = oldSeeds;
    const stock = [self, other, { ...other, id: 'c'.repeat(40), host: '8.1.1.3' }];
    const node = { id: self.id };
    defaults.seeds = stock;
    try {
      expect(getDefaultSeedUpdates({
        historical: config(oldSeeds, { node }),
        installed: config(stock, { node }),
        filtered: config(stock.slice(1), { node }),
        custom: config([self, { ...other, port: 12345 }], { node }),
        unset: config(oldSeeds, { node: { id: null } }),
        absent: config(oldSeeds),
      })).to.deep.equal([
        ['historical', stock.slice(1)],
        ['installed', stock.slice(1)],
        ['unset', stock],
        ['absent', stock],
      ]);
    } finally {
      defaults.seeds = originalSeeds;
    }
  });

  it('should preserve custom, partial, empty, absent, and other-network lists', () => {
    expect(getDefaultSeedUpdates({
      custom: config([...oldSeeds, { ...oldSeeds[0], port: 12345 }]),
      partial: config([oldSeeds[0], oldSeeds[0]]),
      empty: config([]),
      absent: config(undefined),
      testnet: config(oldSeeds, { network: 'testnet' }),
      local: config(oldSeeds, { network: 'local' }),
      current: config(tenderdashSeeds.mainnet.seeds),
    })).to.deep.equal([]);
  });
});
