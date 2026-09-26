import fs from 'fs';
import { checkSnapshot, fetchSnapshot, MAX_AGE_SECONDS } from './tenderdash-seeds.js';

const file = new URL('../configs/defaults/tenderdash-seeds.json', import.meta.url);
const snapshots = JSON.parse(fs.readFileSync(file, 'utf8'));
const { version } = JSON.parse(fs.readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const [mode, ...rest] = process.argv.slice(2);
if (rest.length > 0 || (mode !== undefined && !['--check', '--reuse-snapshot'].includes(mode))) {
  throw new Error('Usage: node scripts/generate-tenderdash-seeds.js [--check | --reuse-snapshot]');
}

if (mode === '--check' && snapshots.version !== version) {
  throw new Error('Seed snapshot version does not match the package; regenerate before releasing');
}

for (const network of ['mainnet', 'testnet']) {
  if (mode === '--check') {
    checkSnapshot(snapshots[network], network);
  } else if (mode === '--reuse-snapshot') {
    // Emergency path for a quorum-server outage during release preparation:
    // carry the last validated snapshot forward untouched. It must still pass
    // the same provenance and age check the publishing workflow applies, so
    // the two can never disagree about what is releasable.
    try {
      checkSnapshot(snapshots[network], network);
    } catch (error) {
      throw new Error(`${error.message}. The committed snapshot cannot be reused; the quorum servers must be reachable`);
    }
  } else {
    snapshots[network] = await fetchSnapshot(network, snapshots[network]);
  }
  process.stdout.write(`${network}: ${snapshots[network].seeds.length} seeds from ${snapshots[network].source}\n`);
}

if (mode === '--reuse-snapshot') {
  const oldest = Math.min(snapshots.mainnet.lastUpdated, snapshots.testnet.lastUpdated);
  const deadline = new Date((oldest + MAX_AGE_SECONDS) * 1000).toISOString();
  process.stderr.write(`WARNING: reusing the ${snapshots.version} seed snapshot for ${version} without refreshing it; `
    + `its data dates from ${new Date(oldest * 1000).toISOString()} and the release must be published before ${deadline}\n`);
}

// Publish neither network until both succeeded. The release commits this file before building.
// Reuse rewrites only the release association; peers, provenance and timestamps stay as validated.
if (mode !== '--check') {
  snapshots.version = version;
  fs.writeFileSync(new URL(`${file.href}.tmp`), `${JSON.stringify(snapshots, null, 2)}\n`);
  fs.renameSync(new URL(`${file.href}.tmp`), file);
}
