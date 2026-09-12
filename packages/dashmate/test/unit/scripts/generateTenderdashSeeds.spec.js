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
  let command;

  function run(args = [], env = {}) {
    return spawnSync(process.execPath, ['--import', path.join(dir, 'fetch.js'), path.join(dir, 'scripts/generate-tenderdash-seeds.js'), ...args], {
      encoding: 'utf8',
      env: {
        ...process.env,
        DASHMATE_MAINNET_CLI: JSON.stringify(command),
        DASHMATE_TESTNET_CLI: JSON.stringify(command),
        ...env,
      },
    });
  }

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-seed-generator-'));
    for (const file of [
      'scripts/generate-tenderdash-seeds.js', 'scripts/tenderdashSeeds.js', 'src/tenderdash/seedSetHash.js',
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
    const fixture = path.join(dir, 'rpc.js');
    fs.writeFileSync(fixture, `
      const method = process.argv[2];
      if (method === 'getblockchaininfo') {
        console.log(JSON.stringify({ chain: 'main', blocks: 100, headers: 100,
          initialblockdownload: false, time: Math.floor(Date.now() / 1000), bestblockhash: 'a'.repeat(64) }));
      } else if (method === 'getblockhash') {
        console.log('a'.repeat(64));
      } else {
        console.log(JSON.stringify(Array.from({length: 5}, (_, i) => ({type: 'Evo', state: {
          platformNodeID: (i + 1).toString(16).padStart(40, '0'), service: '8.1.1.' + (i + 1) + ':9999',
          platformP2PPort: 26656, PoSeBanHeight: -1,
        }}))));
      }
    `);
    command = [process.execPath, fixture];
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

  it('should not write either snapshot when the second network fails', () => {
    const result = run(); // The fixture returns mainnet for both networks.
    expect(result.status).not.to.equal(0);
    expect(result.stdout).to.include('mainnet: 5 seeds');
    expect(result.stderr).to.include('testnet: Core must be synced');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
    expect(fs.existsSync(`${snapshotPath}.tmp`)).to.equal(false);
  });

  it('should refuse to publish a snapshot for another package version', () => {
    const result = run(['--check']);
    expect(result.status).not.to.equal(0);
    expect(result.stderr).to.include('version does not match');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
  });

  it('should not disclose malformed command configuration or RPC stderr', () => {
    const secret = 'test-secret-do-not-log';
    const malformed = run([], { DASHMATE_MAINNET_CLI: secret });
    expect(malformed.status).not.to.equal(0);
    expect(malformed.stderr).not.to.include(secret);
    fs.writeFileSync(command[1], `console.error('${secret}'); process.exit(1);`);
    const failed = run();
    expect(failed.status).not.to.equal(0);
    expect(failed.stderr).to.include('Core RPC getblockchaininfo failed');
    expect(failed.stderr).not.to.include(secret);
  });

  it('should generate both networks from quorum servers with no CLI configuration', () => {
    const env = { DASHMATE_MAINNET_CLI: undefined, DASHMATE_TESTNET_CLI: undefined };
    const result = run([], env);
    expect(result.status, result.stderr).to.equal(0);
    const snapshots = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    expect(snapshots.version).to.equal('5.0.0');
    for (const network of ['mainnet', 'testnet']) {
      expect(snapshots[network].source).to.equal(`https://quorums.${network}.networks.dash.org/masternodes`);
      expect(snapshots[network].seeds).to.have.length(5);
      expect(snapshots[network]).not.to.have.property('blockHash');
    }
    expect(snapshots.testnet.seeds[0].port).to.equal(36656);
    // Publishing checks the committed data offline, even with no CLI overrides.
    const check = run(['--check'], { ...env, SEED_TEST_FETCH_MODE: 'forbid' });
    expect(check.status, check.stderr).to.equal(0);
  });

  it('should preserve both snapshots when the second quorum server fails', () => {
    const result = run([], {
      DASHMATE_MAINNET_CLI: undefined,
      DASHMATE_TESTNET_CLI: undefined,
      SEED_TEST_FETCH_MODE: 'fail-testnet',
    });
    expect(result.status).not.to.equal(0);
    expect(result.stdout).to.include('mainnet: 5 seeds');
    expect(result.stderr).to.include('HTTP 503');
    expect(fs.readFileSync(snapshotPath, 'utf8')).to.equal(original);
  });

  it('should allow a Core override for one network and the quorum server for the other', () => {
    const result = run([], { DASHMATE_TESTNET_CLI: undefined });
    expect(result.status, result.stderr).to.equal(0);
    const snapshots = JSON.parse(fs.readFileSync(snapshotPath, 'utf8'));
    expect(snapshots.mainnet.blockHash).to.equal('a'.repeat(64));
    expect(snapshots.mainnet).not.to.have.property('source');
    expect(snapshots.testnet.source).to.include('quorums.testnet');
  });
});
