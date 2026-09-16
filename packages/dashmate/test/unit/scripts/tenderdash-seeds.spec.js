import crypto from 'crypto';
import {
  generateSnapshot, fetchSnapshot, checkSnapshot, QUORUM_URLS,
} from '../../../scripts/tenderdash-seeds.js';
import seedSetHash from '../../../src/tenderdash/seedSetHash.js';

const NOW = 1800000000;

describe('quorum-server Tenderdash seeds', () => {
  let response;
  let previous;

  beforeEach(() => {
    response = {
      success: true,
      message: null,
      lastUpdated: NOW - 60,
      data: Array.from({ length: 25 }, (_, index) => ({
        proTxHash: (index + 1).toString(16).padStart(64, '0'),
        platformNodeID: (index + 1).toString(16).padStart(40, '0'),
        address: `8.1.1.${index + 1}:9999`,
        platformP2PPort: 26656,
        platformHTTPPort: 443,
        status: 'ENABLED',
        versionCheck: 'success',
      })),
    };
    previous = {
      seeds: [{ id: 'f'.repeat(40), host: '8.9.1.1', port: 1234 }],
      previousSeedSetHashes: ['a'.repeat(64)],
    };
  });

  it('should preserve separate registered endpoints and record server provenance', () => {
    response.data = response.data.slice(0, 20);
    response.data[0].addresses = { platform_p2p: ['9.1.1.1:27656', '[2001:db8::1]:27656'] };
    const snapshot = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(snapshot.seeds).to.have.length(20);
    expect(snapshot.seeds[0]).to.deep.equal({ id: '1'.padStart(40, '0'), host: '9.1.1.1', port: 27656 });
    expect(snapshot).to.include({ source: QUORUM_URLS.mainnet, lastUpdated: NOW - 60 });
    expect(snapshot).not.to.have.property('height');
    expect(snapshot).not.to.have.property('blockHash');
    expect(snapshot).not.to.have.property('blockTime');
    expect(snapshot.previousSeedSetHashes).to.include(seedSetHash(previous.seeds));
    expect(snapshot.previousSeedSetHashes).to.include('a'.repeat(64));
    response.data.reverse();
    expect(generateSnapshot(response, 'mainnet', previous, NOW)).to.deep.equal(snapshot);
  });

  it('should sample membership before sorting and give each identity one chance', function samplePeers() {
    const random = this.sinon.stub(crypto, 'randomInt').callsFake(max => max - 1);
    const first = generateSnapshot(response, 'mainnet', previous, NOW);
    random.callsFake(() => 0);
    const second = generateSnapshot(response, 'mainnet', first, NOW);
    expect(second.seeds).not.to.deep.equal(first.seeds);
    expect(second.seeds).to.deep.include({ id: '15'.padStart(40, '0'), host: '8.1.1.21', port: 26656 });
    expect(second.previousSeedSetHashes).to.include(seedSetHash(first.seeds));
    response.data.push(response.data[0]);
    response.data[0].addresses = { platform_p2p: ['8.1.1.1:26656', '9.1.1.1:26656'] };
    random.resetHistory();
    expect(generateSnapshot(response, 'mainnet', first, NOW)).to.deep.equal(second);
    expect(random.callCount).to.equal(24);
  });

  it('should use an alternate registered endpoint when peers share a primary host', () => {
    response.data = response.data.slice(0, 5).map((node, index) => ({
      ...node,
      addresses: { platform_p2p: ['9.1.1.1:26656', `8.1.1.${index + 1}:26656`] },
    }));
    const { seeds } = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(seeds).to.have.length(5);
    expect(new Set(seeds.map(seed => seed.host)).size).to.equal(5);
  });

  it('should move an earlier pick to its alternate host instead of discarding a constrained peer', function matchHosts() {
    // Sampling order is fixed to registry order: randomInt(max) = max - 1 swaps each index with itself.
    this.sinon.stub(crypto, 'randomInt').callsFake(max => max - 1);
    response.data = response.data.slice(0, 5);
    // A lists X and Y and is sampled first; B lists only X. Greedy first-free-host
    // gives A host X, skips B, and fails the five-peer minimum.
    response.data[0].addresses = { platform_p2p: ['9.1.1.1:26656', '9.1.1.2:26656'] };
    response.data[1].addresses = { platform_p2p: ['9.1.1.1:26656'] };
    const { seeds } = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(seeds).to.have.length(5);
    expect(new Set(seeds.map(seed => seed.host)).size).to.equal(5);
    expect(seeds).to.deep.include({ id: '1'.padStart(40, '0'), host: '9.1.1.2', port: 26656 });
    expect(seeds).to.deep.include({ id: '2'.padStart(40, '0'), host: '9.1.1.1', port: 26656 });
    // The constrained peer first is the easy order; both orders must select the same identities.
    response.data.reverse();
    const reversed = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(reversed.seeds.map(seed => seed.id)).to.deep.equal(seeds.map(seed => seed.id));
  });

  it('should keep the sampled slot order when a chain of alternates has to shift', function chainHosts() {
    this.sinon.stub(crypto, 'randomInt').callsFake(max => max - 1);
    // Identity i (1-based) can use hosts i and i+1; the last one only host 5.
    // Every earlier identity has to shift one host over for all five to fit.
    response.data = response.data.slice(0, 5).map((node, index) => ({
      ...node,
      addresses: { platform_p2p: index === 4 ? ['9.1.1.5:26656'] : [`9.1.1.${index + 1}:26656`, `9.1.1.${index + 2}:26656`] },
    }));
    // Add a sixth identity that must not be picked over the constrained one.
    // It is sampled last, so it only gets a slot if one is still free; with
    // MAX_SEEDS at 20 and six identities, all six fit on distinct hosts.
    response.data.push({ ...response.data[0], platformNodeID: '6'.padStart(40, '0'), addresses: { platform_p2p: ['9.1.1.9:26656'] } });
    const { seeds } = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(seeds).to.have.length(6);
    expect(new Set(seeds.map(seed => seed.host)).size).to.equal(6);
    expect(seeds.find(seed => seed.id === '5'.padStart(40, '0')).host).to.equal('9.1.1.5');
  });

  it('should exclude banned, failed, incomplete and malformed peers without guessing ports', () => {
    response.data[0].status = 'POSE_BANNED';
    response.data[1].versionCheck = 'fail';
    delete response.data[2].platformNodeID;
    delete response.data[3].platformP2PPort;
    response.data[4].addresses = { platform_p2p: ['bad', null, '9.1.1.1:99999'] };
    response.data[5].platformNodeID = '0'.repeat(40);
    response.data[6].addresses = { platform_p2p: [] };
    response.data.push(null, response.data[7]);
    const { seeds } = generateSnapshot(response, 'mainnet', previous, NOW);
    expect(seeds).to.have.length(18);
    expect(new Set(seeds.map(seed => seed.id)).size).to.equal(18);
    expect(new Set(seeds.map(seed => seed.host)).size).to.equal(18);
  });

  it('should reject unavailable, old-server, stale and future-dated registry responses', () => {
    [
      null, {}, { ...response, success: false }, { ...response, data: {} },
      { ...response, lastUpdated: undefined }, { ...response, lastUpdated: NOW - 1801 },
      { ...response, lastUpdated: NOW + 301 }, { ...response, lastUpdated: '1800000000' },
    ].forEach(invalid => {
      expect(() => generateSnapshot(invalid, 'mainnet', previous, NOW)).to.throw('bootstrap metadata');
    });
  });

  it('should reject old server responses without Platform node IDs', () => {
    response.data = response.data.map(({ address, status, versionCheck }) => ({ address, status, versionCheck }));
    expect(() => generateSnapshot(response, 'mainnet', previous, NOW)).to.throw('platformNodeID');
  });

  it('should reject too few candidates and preserve the previously committed snapshot', () => {
    response.data = response.data.slice(0, 4);
    const original = structuredClone(previous);
    expect(() => generateSnapshot(response, 'mainnet', previous, NOW)).to.throw('at least 5');
    expect(previous).to.deep.equal(original);
  });

  it('should enforce source network and freshness when checking a release offline', () => {
    const snapshot = generateSnapshot(response, 'testnet', previous, NOW);
    expect(() => checkSnapshot(snapshot, 'testnet', NOW)).not.to.throw();
    expect(() => checkSnapshot(snapshot, 'mainnet', NOW)).to.throw();
    expect(() => checkSnapshot({ ...snapshot, source: QUORUM_URLS.mainnet }, 'testnet', NOW)).to.throw();
    expect(() => checkSnapshot(snapshot, 'testnet', NOW + 7 * 86400)).to.throw('stale');
    expect(() => checkSnapshot({ ...snapshot, lastUpdated: undefined }, 'testnet', NOW)).to.throw();
  });

  it('should reject malformed snapshots and Core provenance when publishing', () => {
    const snapshot = generateSnapshot(response, 'mainnet', previous, NOW);
    for (const change of [
      { source: undefined, chain: 'main', height: 100, blockHash: 'a'.repeat(64), blockTime: NOW },
      { lastUpdated: NOW + 301 }, { seeds: [] }, { seeds: [null] },
      { seeds: [...snapshot.seeds.slice(1), snapshot.seeds[1]] },
      { seeds: snapshot.seeds.map(seed => ({ ...seed, port: 0 })) },
      { previousSeedSetHashes: ['invalid'] },
    ]) {
      expect(() => checkSnapshot({ ...snapshot, ...change }, 'mainnet', NOW)).to.throw();
    }
  });

  it('should fetch each network endpoint with a timeout and no redirects', async function fetchBothNetworks() {
    const fetch = this.sinon.stub().resolves({ ok: true, json: async () => response });
    for (const network of ['mainnet', 'testnet']) {
      const result = await fetchSnapshot(network, previous, fetch, NOW);
      expect(result.source).to.equal(QUORUM_URLS[network]);
      expect(fetch.lastCall.args[0]).to.equal(QUORUM_URLS[network]);
      expect(fetch.lastCall.args[1].redirect).to.equal('error');
      expect(fetch.lastCall.args[1].signal).to.be.instanceOf(AbortSignal);
    }
  });

  it('should surface HTTP, transport, timeout and malformed JSON failures', async function rejectFetchFailures() {
    const fetch = this.sinon.stub();
    fetch.resolves({ ok: false, status: 503 });
    await expect(fetchSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('HTTP 503');
    fetch.rejects(new Error('request timed out'));
    await expect(fetchSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('request timed out');
    fetch.resolves({ ok: true, json: async () => { throw new Error('invalid JSON'); } });
    await expect(fetchSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('invalid JSON');
  });
});
