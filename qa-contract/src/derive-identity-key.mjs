// Recover the QA identity's signing key from a known wallet mnemonic.
//
// Use this when the identity was registered by a wallet whose mnemonic you
// control (e.g. registered via SwiftExampleApp / platform-wallet, which derives
// identity authentication keys at the DIP13 path
//   m/9'/<coin>'/5'/0'/<keyType>'/<identityIndex>'/<keyIndex>'
// — coin = 1 for testnet/devnet/local, 5 for mainnet; keyType 0 = ECDSA).
//
// It fetches the on-chain identity, derives candidate keys from the mnemonic,
// matches them to the identity's public keys, and prints (or writes to .env) the
// WIF + key id of a writable HIGH/CRITICAL AUTHENTICATION key suitable for
// signing contract/document transitions.
//
// Usage:
//   QA_MNEMONIC="..." QA_IDENTITY_ID=... node src/derive-identity-key.mjs [--write] [--print]
//
// The recovered WIF is masked by default; pass --print to echo it in full.
// --write saves it to .env and chmods the file to 0600.

import { parseArgs } from 'node:util';
import {
  existsSync, readFileSync, writeFileSync, chmodSync,
} from 'node:fs';
import { join } from 'node:path';
import { loadDotEnv, connect, sdkNetwork, QA_DIR } from './sdk.mjs';

const COIN = (net) => (net === 'mainnet' ? 5 : 1);
const SEC_RANK = { CRITICAL: 0, HIGH: 1, MEDIUM: 2, LOW: 3 };

function normHex(data) {
  if (data == null) return undefined;
  if (typeof data === 'string') {
    const s = data.toLowerCase();
    if (/^[0-9a-f]+$/.test(s)) return s;
    try { return Buffer.from(data, 'base64').toString('hex'); } catch { return s; }
  }
  try { return Buffer.from(data).toString('hex'); } catch { return undefined; }
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      write: { type: 'boolean', default: false },
      print: { type: 'boolean', default: false },
    },
  });

  const mnemonic = (process.env.QA_MNEMONIC || '').trim();
  const identityId = (process.env.QA_IDENTITY_ID || '').trim();
  if (!mnemonic) throw new Error('Set QA_MNEMONIC (the wallet mnemonic that owns the identity).');
  if (!identityId) throw new Error('Set QA_IDENTITY_ID (the registered identity id).');

  const { sdk, mod, network } = await connect();
  const coin = COIN(network);
  const identity = await sdk.identities.fetch(identityId);
  if (!identity) throw new Error(`Identity ${identityId} not found on ${network}.`);

  const onChain = identity.publicKeys.map((k) => ({
    id: Number(k.keyId ?? k.id), purpose: String(k.purpose), securityLevel: String(k.securityLevel),
    keyType: String(k.keyType), readOnly: !!k.isReadOnly, hex: normHex(k.data),
  }));

  const matches = [];
  for (let idIdx = 0; idIdx <= 3; idIdx += 1) {
    for (let kt = 0; kt <= 1; kt += 1) {
      for (let ki = 0; ki <= 6; ki += 1) {
        const path = `m/9'/${coin}'/5'/0'/${kt}'/${idIdx}'/${ki}'`;
        let d;
        try { d = await mod.wallet.deriveKeyFromSeedWithPath({ mnemonic, path, network: sdkNetwork(network) }); } catch { continue; }
        const pub = d.publicKey.toLowerCase();
        const hit = onChain.find((k) => k.hex === pub);
        if (hit) matches.push({ ...hit, path, wif: d.privateKeyWif });
      }
    }
  }

  if (!matches.length) {
    console.error('No derived key matched any on-chain key. On-chain keys:');
    for (const k of onChain) console.error(`  id=${k.id} ${k.purpose}/${k.securityLevel} ${k.keyType} ro=${k.readOnly}`);
    process.exit(2);
  }

  console.log(`Matched ${matches.length} key(s) on identity ${identityId}:`);
  for (const m of matches) console.log(`  id=${m.id} ${m.purpose}/${m.securityLevel} ${m.keyType} ro=${m.readOnly}  ${m.path}`);

  const signable = matches
    .filter((m) => m.purpose === 'AUTHENTICATION' && !m.readOnly && (m.securityLevel === 'HIGH' || m.securityLevel === 'CRITICAL'))
    .sort((a, b) => SEC_RANK[a.securityLevel] - SEC_RANK[b.securityLevel] || a.id - b.id);
  // Prefer HIGH (matches the platform doc/contract transition precedent), else CRITICAL.
  const pick = signable.find((m) => m.securityLevel === 'HIGH') || signable[0];
  if (!pick) throw new Error('No writable HIGH/CRITICAL AUTHENTICATION key matched.');

  const maskedWif = `${pick.wif.slice(0, 4)}…${pick.wif.slice(-4)}`;
  console.log(`\nSigning key: id=${pick.id} ${pick.securityLevel} AUTHENTICATION`);
  console.log(`  WIF: ${values.print ? pick.wif : maskedWif}${values.print ? '' : '  (masked — pass --print to reveal)'}`);

  if (values.write) {
    const envPath = join(QA_DIR, '.env');
    let env = existsSync(envPath) ? readFileSync(envPath, 'utf8') : '';
    const setLine = (key, val) => {
      const re = new RegExp(`^${key}=.*$`, 'm');
      env = re.test(env) ? env.replace(re, `${key}=${val}`) : `${env}\n${key}=${val}`;
    };
    setLine('QA_PRIVATE_KEY', pick.wif);
    if (Number.isFinite(pick.id)) setLine('QA_IDENTITY_KEY_ID', String(pick.id));
    // mode on writeFileSync applies only when creating the file (closes the
    // create-at-0644-then-chmod window); chmodSync covers the overwrite case.
    writeFileSync(envPath, env.endsWith('\n') ? env : `${env}\n`, { mode: 0o600 });
    chmodSync(envPath, 0o600); // contains a private key + mnemonic — owner-only
    console.log(`\nWrote QA_PRIVATE_KEY${Number.isFinite(pick.id) ? ' + QA_IDENTITY_KEY_ID' : ''} to ${envPath} (chmod 0600).`);
  } else {
    console.log('\nRe-run with --write to save QA_PRIVATE_KEY + QA_IDENTITY_KEY_ID into .env (chmod 0600).');
  }
}

main().catch((e) => { console.error('derive-identity-key failed:', e?.stack || e); process.exit(1); });
