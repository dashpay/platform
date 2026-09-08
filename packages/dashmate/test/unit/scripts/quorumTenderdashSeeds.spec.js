import {
  generateQuorumSnapshot, fetchQuorumSnapshot, checkSnapshot, QUORUM_URLS,
} from '../../../scripts/tenderdashSeeds.js';
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
    response.data[0].addresses = { platform_p2p: ['9.1.1.1:27656', '[2001:db8::1]:27656'] };
    const snapshot = generateQuorumSnapshot(response, 'mainnet', previous, NOW);
    expect(snapshot.seeds).to.have.length(20);
    expect(snapshot.seeds[0]).to.deep.equal({ id: '1'.padStart(40, '0'), host: '9.1.1.1', port: 27656 });
    expect(snapshot).to.include({ chain: 'main', source: QUORUM_URLS.mainnet, lastUpdated: NOW - 60 });
    expect(snapshot).not.to.have.property('height');
    expect(snapshot).not.to.have.property('blockHash');
    expect(snapshot).not.to.have.property('blockTime');
    expect(snapshot.previousSeedSetHashes).to.include(seedSetHash(previous.seeds));
    expect(snapshot.previousSeedSetHashes).to.include('a'.repeat(64));
    response.data.reverse();
    expect(generateQuorumSnapshot(response, 'mainnet', previous, NOW)).to.deep.equal(snapshot);
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
    const { seeds } = generateQuorumSnapshot(response, 'mainnet', previous, NOW);
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
      expect(() => generateQuorumSnapshot(invalid, 'mainnet', previous, NOW)).to.throw('bootstrap metadata');
    });
  });

  it('should reject old server responses without Platform node IDs', () => {
    response.data = response.data.map(({ address, status, versionCheck }) => ({ address, status, versionCheck }));
    expect(() => generateQuorumSnapshot(response, 'mainnet', previous, NOW)).to.throw('platformNodeID');
  });

  it('should reject too few candidates and preserve the previously committed snapshot', () => {
    response.data = response.data.slice(0, 4);
    const original = structuredClone(previous);
    expect(() => generateQuorumSnapshot(response, 'mainnet', previous, NOW)).to.throw('at least 5');
    expect(previous).to.deep.equal(original);
  });

  it('should enforce source network and freshness when checking a release offline', () => {
    const snapshot = generateQuorumSnapshot(response, 'testnet', previous, NOW);
    expect(() => checkSnapshot(snapshot, 'testnet', NOW)).not.to.throw();
    expect(() => checkSnapshot(snapshot, 'mainnet', NOW)).to.throw();
    expect(() => checkSnapshot({ ...snapshot, source: QUORUM_URLS.mainnet }, 'testnet', NOW)).to.throw();
    expect(() => checkSnapshot(snapshot, 'testnet', NOW + 7 * 86400)).to.throw('stale');
    expect(() => checkSnapshot({ ...snapshot, lastUpdated: undefined }, 'testnet', NOW)).to.throw();
  });

  it('should fetch each network endpoint with a timeout and no redirects', async function fetchBothNetworks() {
    const fetch = this.sinon.stub().resolves({ ok: true, json: async () => response });
    for (const network of ['mainnet', 'testnet']) {
      const result = await fetchQuorumSnapshot(network, previous, fetch, NOW);
      expect(result.source).to.equal(QUORUM_URLS[network]);
      expect(fetch.lastCall.args[0]).to.equal(QUORUM_URLS[network]);
      expect(fetch.lastCall.args[1].redirect).to.equal('error');
      expect(fetch.lastCall.args[1].signal).to.be.instanceOf(AbortSignal);
    }
  });

  it('should surface HTTP, transport, timeout and malformed JSON failures', async function rejectFetchFailures() {
    const fetch = this.sinon.stub();
    fetch.resolves({ ok: false, status: 503 });
    await expect(fetchQuorumSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('HTTP 503');
    fetch.rejects(new Error('request timed out'));
    await expect(fetchQuorumSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('request timed out');
    fetch.resolves({ ok: true, json: async () => { throw new Error('invalid JSON'); } });
    await expect(fetchQuorumSnapshot('mainnet', previous, fetch, NOW)).to.be.rejectedWith('invalid JSON');
  });
});
