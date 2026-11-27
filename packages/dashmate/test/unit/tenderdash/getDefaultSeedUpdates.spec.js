import fs from 'fs';
import tenderdashSeeds from '../../../configs/defaults/tenderdashSeeds.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import getDefaultSeedUpdates from '../../../src/tenderdash/getDefaultSeedUpdates.js';
import seedSetHash from '../../../src/tenderdash/seedSetHash.js';

// From the published Dashmate 5.1.0-rc.1 snapshot (fd256502065).
const rc1Seeds = JSON.parse(fs.readFileSync(
  new URL('../../fixtures/tenderdash/5.1.0-rc.1-seeds.json', import.meta.url),
  'utf8',
));
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

  it('should save the full stock list on refresh and restore an omitted local ID', () => {
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
        ['historical', stock],
        ['filtered', stock],
        ['unset', stock],
        ['absent', stock],
      ]);
    } finally {
      defaults.seeds = originalSeeds;
    }
  });

  it('should recognize an omitted ID derived from the key when the saved ID is stale', () => {
    const key = generateTenderdashNodeKey();
    const self = { ...oldSeeds[0], id: deriveTenderdashNodeId(key).toUpperCase() };
    const stale = oldSeeds[1];
    const stock = [self, stale];
    const originalSeeds = tenderdashSeeds.mainnet.seeds;
    tenderdashSeeds.mainnet.seeds = stock;
    try {
      const node = { key, id: stale.id };
      expect(getDefaultSeedUpdates({
        filtered: config([stale], { node }),
        installed: config(stock, { node }),
      })).to.deep.equal([['filtered', stock]]);
    } finally {
      tenderdashSeeds.mainnet.seeds = originalSeeds;
    }
  });

  it('should use the canonical saved ID for filtered history when the key is invalid', () => {
    const [self, peer] = oldSeeds;
    tenderdashSeeds.mainnet.previousSeedSetHashes.push(seedSetHash([peer], self.id));
    const node = { id: self.id.toUpperCase(), key: 'invalid' };

    expect(getDefaultSeedUpdates({ filtered: config([peer], { node }) }))
      .to.deep.equal([['filtered', tenderdashSeeds.mainnet.seeds]]);
  });

  it('should restore a legacy stock list using either the derived or the saved ID', () => {
    const key = generateTenderdashNodeKey();
    const keyPeer = { ...oldSeeds[0], id: deriveTenderdashNodeId(key) };
    const savedPeer = oldSeeds[1];
    const node = { key, id: savedPeer.id.toUpperCase() };
    tenderdashSeeds.mainnet.previousSeedSetHashes.push(
      seedSetHash([savedPeer], keyPeer.id),
      seedSetHash([keyPeer], savedPeer.id),
    );

    const current = tenderdashSeeds.mainnet.seeds;
    expect(getDefaultSeedUpdates({
      derived: config([savedPeer], { node }),
      saved: config([keyPeer], { node }),
    })).to.deep.equal([['derived', current], ['saved', current]]);
  });

  for (const [network, entries] of Object.entries(rc1Seeds)) {
    const seeds = entries.map(([id, host, port]) => ({ id, host, port }));
    // Prefer a node still present in current defaults so the repair must restore its entry.
    const current = tenderdashSeeds[network].seeds;
    const self = seeds.find(({ id }) => current.some(seed => seed.id === id)) ?? seeds[0];
    const removedOther = seeds.find(seed => seed !== self);
    const withoutSelf = seeds.filter(seed => seed !== self);

    it(`should refresh ${network} 5.1.0-rc.1 stock seeds with only self removed`, () => {
      expect(getDefaultSeedUpdates({
        repaired: config(withoutSelf, { network, node: { id: self.id } }),
      })).to.deep.equal([['repaired', current]]);
    });

    it(`should preserve other partial ${network} 5.1.0-rc.1 lists`, () => {
      expect(getDefaultSeedUpdates({
        removedOther: config(seeds.filter(seed => seed !== removedOther), {
          network, node: { id: self.id },
        }),
        absentOwner: config(withoutSelf, { network, node: { id: 'f'.repeat(40) } }),
      })).to.deep.equal([]);
    });
  }

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
