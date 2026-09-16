import fs from 'fs';
import { checkSnapshot, fetchSnapshot } from './tenderdash-seeds.js';

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
  } else {
    snapshots[network] = await fetchSnapshot(network, snapshots[network]);
  }
  process.stdout.write(`${network}: ${snapshots[network].seeds.length} seeds from ${snapshots[network].source}\n`);
}

// Publish neither network until both succeeded. The release commits this file before building.
if (args[0] !== '--check') {
  snapshots.version = version;
  fs.writeFileSync(new URL(`${file.href}.tmp`), `${JSON.stringify(snapshots, null, 2)}\n`);
  fs.renameSync(new URL(`${file.href}.tmp`), file);
}
