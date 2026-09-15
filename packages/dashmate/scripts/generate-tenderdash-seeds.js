import fs from 'fs';
import { execFileSync } from 'child_process';
import { checkSnapshot, generateSnapshot, fetchQuorumSnapshot } from './tenderdashSeeds.js';

const file = new URL('../configs/defaults/tenderdash-seeds.json', import.meta.url);
const snapshots = JSON.parse(fs.readFileSync(file, 'utf8'));
const { version } = JSON.parse(fs.readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const args = process.argv.slice(2);
if (args.length > 1 || (args.length === 1 && args[0] !== '--check')) {
  throw new Error('Usage: node scripts/generate-tenderdash-seeds.js [--check]');
}

if (args[0] === '--check' && snapshots.version !== version) {
  throw new Error('Seed snapshot version does not match the package; regenerate before releasing');
}

for (const network of ['mainnet', 'testnet']) {
  if (args[0] === '--check') {
    checkSnapshot(snapshots[network], network);
  } else if (process.env[`DASHMATE_${network.toUpperCase()}_CLI`] === undefined) {
    snapshots[network] = await fetchQuorumSnapshot(network, snapshots[network]);
  } else {
    const variable = `DASHMATE_${network.toUpperCase()}_CLI`;
    // JSON argv supports local dash-cli, docker exec, and SSH without shell evaluation.
    let command;
    try {
      command = JSON.parse(process.env[variable] || 'null');
    } catch {
      throw new Error(`${variable} must be a JSON argv array`);
    }
    if (!Array.isArray(command) || command.length === 0
      || command.some(arg => typeof arg !== 'string' || arg.length === 0)) {
      throw new Error(`Set ${variable} to a JSON argv array for a synced dash-cli (see docs/tenderdash-seeds.md)`);
    }
    const rpc = (...rpcArgs) => {
      try {
        const output = execFileSync(command[0], [...command.slice(1), ...rpcArgs], {
          encoding: 'utf8',
          timeout: 60000,
          maxBuffer: 16 * 1024 * 1024,
          stdio: ['ignore', 'pipe', 'pipe'],
        }).trim();
        return rpcArgs[0] === 'getblockhash' ? output : JSON.parse(output);
      } catch {
        // Do not echo commands, credentials, or remote stderr into release logs.
        throw new Error(`${network}: Core RPC ${rpcArgs[0]} failed`);
      }
    };
    snapshots[network] = generateSnapshot(rpc, network, snapshots[network]);
  }
  const source = snapshots[network].source
    ? `quorum registry updated at ${snapshots[network].lastUpdated}`
    : `Core block ${snapshots[network].height}`;
  process.stdout.write(`${network}: ${snapshots[network].seeds.length} seeds from ${source}\n`);
}

// Publish neither network until both succeeded. The release commits this file before building.
if (args[0] !== '--check') {
  snapshots.version = version;
  fs.writeFileSync(new URL(`${file.href}.tmp`), `${JSON.stringify(snapshots, null, 2)}\n`);
  fs.renameSync(new URL(`${file.href}.tmp`), file);
}
