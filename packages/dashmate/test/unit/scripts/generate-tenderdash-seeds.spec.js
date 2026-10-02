import fs from 'fs';
import os from 'os';
import path from 'path';
import { spawnSync } from 'child_process';
import { fileURLToPath } from 'url';

const packageRoot = fileURLToPath(new URL('../../../', import.meta.url));

describe('generate-tenderdash-seeds CLI', () => {
  let dir;
  let snapshotPath;
  let original;

  function run(args = [], env = {}) {
    return spawnSync(process.execPath, ['--import', path.join(dir, 'fetch.js'), path.join(dir, 'scripts/generate-tenderdash-seeds.js'), ...args], {
      encoding: 'utf8',
      env: {
        ...process.env,
        ...env,
      },
    });
  }

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-seed-generator-'));
    for (const file of [
      'scripts/generate-tenderdash-seeds.js', 'scripts/tenderdash-seeds.js', 'src/tenderdash/seedSetHash.js',
    ]) {
      const target = path.join(dir, file);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.copyFileSync(path.join(packageRoot, file), target);
    }
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ type: 'module', version: '5.0.0' }));
    snapshotPath = path.join(dir, 'configs/defaults/tenderdash-seeds.json');
    fs.mkdirSync(path.dirname(snapshotPath), { recursive: true });
    const snapshot = { seeds: [], previousSeedSetHashes: [] };
    original = JSON.stringify({ mainnet: snapshot, testnet: snapshot, version: '4.0.0' });
    fs.writeFileSync(snapshotPath, original);
    fs.writeFileSync(path.join(dir, 'fetch.js'), `
      globalThis.fetch = async (url, options) => {
        const network = url === 'https://quorums.mainnet.networks.dash.org/masternodes' ? 'mainnet'
          : url === 'https://quorums.testnet.networks.dash.org/masternodes' ? 'testnet' : null;
        if (!network || options.redirect !== 'error' || !options.signal) throw new Error('Unexpected HTTP request');
        if (process.env.SEED_TEST_FETCH_MODE === 'fail-testnet' && network === 'testnet') {
          return { ok: false, status: 503 };
        }
        if (process.env.SEED_TEST_FETCH_MODE === 'forbid') throw new Error('HTTP must not be called');
        return { ok: true, json: async () => ({success: true, lastUpdated: Math.floor(Date.now() / 1000),
          data: Array.from({length: 5}, (_, i) => ({
            platformNodeID: (i + 1).toString(16).padStart(40, '0'),
            address: '8.1.1.' + (i + 1) + ':9999',
            status: 'ENABLED', versionCheck: 'success',
            platformP2PPort: network === 'mainnet' ? 26656 : 36656,
          }))
        })};
      };
    `);
  });

  afterEach(() => fs.rmSync(dir, { recursive: true, force: true }));

  it('should refuse to publish a snapshot for another package version', () => {
    const result = run(['--check']);
    expect(result.status).not.to.equal(0);
    expect(result.stderr).to.include('version does not match');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
  });

  it('should generate both networks from quorum servers and validate offline', () => {
    const result = run();
    expect(result.status, result.stderr).to.equal(0);
    const snapshots = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    expect(snapshots.version).to.equal('5.0.0');
    for (const network of ['mainnet', 'testnet']) {
      expect(snapshots[network].source).to.equal(`https://quorums.${network}.networks.dash.org/masternodes`);
      expect(snapshots[network].seeds).to.have.length(5);
      expect(snapshots[network]).not.to.have.property('blockHash');
    }
    expect(snapshots.testnet.seeds[0].port).to.equal(36656);
    // Publishing checks the committed data offline.
    const check = run(['--check'], { SEED_TEST_FETCH_MODE: 'forbid' });
    expect(check.status, check.stderr).to.equal(0);
  });

  it('should reuse a validated snapshot for a new version without touching its data', () => {
    expect(run().status).to.equal(0);
    const generated = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify({ type: 'module', version: '5.0.1' }));

    const result = run(['--reuse-snapshot'], { SEED_TEST_FETCH_MODE: 'forbid' });
    expect(result.status, result.stderr).to.equal(0);
    expect(result.stderr).to.include('WARNING: reusing the 5.0.0 seed snapshot for 5.0.1');
    expect(result.stderr).to.include('must be published before');
    const reused = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    expect(reused.version).to.equal('5.0.1');
    expect(reused.mainnet).to.deep.equal(generated.mainnet);
    expect(reused.testnet).to.deep.equal(generated.testnet);
    // The publishing check accepts exactly what reuse produced.
    const check = run(['--check'], { SEED_TEST_FETCH_MODE: 'forbid' });
    expect(check.status, check.stderr).to.equal(0);
  });

  it('should refuse to reuse a snapshot the publishing check would reject', () => {
    // The committed Core-derived snapshot has no quorum-server provenance.
    let result = run(['--reuse-snapshot'], { SEED_TEST_FETCH_MODE: 'forbid' });
    expect(result.status).not.to.equal(0);
    expect(result.stderr).to.include('cannot be reused');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);

    // A validated snapshot past the publishing age limit is equally unusable.
    expect(run().status).to.equal(0);
    const snapshots = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    snapshots.testnet.lastUpdated -= 8 * 24 * 60 * 60;
    const stale = `${JSON.stringify(snapshots, null, 2)}\n`;
    fs.writeFileSync(snapshotPath, stale);
    result = run(['--reuse-snapshot'], { SEED_TEST_FETCH_MODE: 'forbid' });
    expect(result.status).not.to.equal(0);
    expect(result.stdout).to.include('mainnet: 5 seeds');
    expect(result.stderr).to.include('stale');
    expect(result.stderr).to.include('cannot be reused');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(stale);
    expect(fs.existsSync(`${snapshotPath}.tmp`)).to.equal(false);
  });

  it('should reject unknown or combined modes', () => {
    for (const args of [['--check', '--reuse-snapshot'], ['--reuse'], ['--reuse-snapshot', 'extra']]) {
      const result = run(args, { SEED_TEST_FETCH_MODE: 'forbid' });
      expect(result.status).not.to.equal(0);
      expect(result.stderr).to.include('Usage');
      expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
    }
  });

  it('should preserve both snapshots when the second quorum server fails', () => {
    const result = run([], {
      SEED_TEST_FETCH_MODE: 'fail-testnet',
    });
    expect(result.status).not.to.equal(0);
    expect(result.stdout).to.include('mainnet: 5 seeds');
    expect(result.stderr).to.include('HTTP 503');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
    expect(fs.existsSync(`${snapshotPath}.tmp`)).to.equal(false);
  });
});
