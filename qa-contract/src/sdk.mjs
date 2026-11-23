// Shared helpers for the QA-contract scripts: load the Evo SDK, connect to a
// network, build a signer from a private key, resolve the signing identity key,
// and read/write the committed contract-id config.
//
// SDK loading order of precedence:
//   1. EVO_SDK_BUNDLE env var -> import that file directly (a prebuilt
//      dist/evo-sdk.module.js). Useful when the workspace package is not built
//      in the current working tree.
//   2. bare import of '@dashevo/evo-sdk' (the normal monorepo path, requires the
//      workspace package to be built: `yarn workspace @dashevo/evo-sdk build`).

import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, resolve, join } from 'node:path';
import { createHash, randomBytes } from 'node:crypto';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const QA_DIR = resolve(__dirname, '..');
export const SCHEMA_PATH = join(QA_DIR, 'schema', 'qa-contract.documents.json');

// ---------------------------------------------------------------------------
// Minimal .env loader (no dependency). Reads qa-contract/.env if present.
// ---------------------------------------------------------------------------
export function loadDotEnv() {
  const envPath = join(QA_DIR, '.env');
  if (!existsSync(envPath)) return;
  for (const raw of readFileSync(envPath, 'utf8').split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq === -1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim();
    if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
      val = val.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = val;
  }
}

// ---------------------------------------------------------------------------
// SDK module loading + connection
// ---------------------------------------------------------------------------
let _modPromise;
export async function loadSdkModule() {
  if (!_modPromise) {
    const bundle = process.env.EVO_SDK_BUNDLE;
    _modPromise = bundle
      ? import(pathToFileURL(resolve(bundle)).href)
      : import('@dashevo/evo-sdk');
  }
  return _modPromise;
}

export function getNetwork() {
  return (process.env.NETWORK || 'testnet').toLowerCase();
}

// Canonical network id (matches the SDK/app Network enum): mainnet=0, testnet=1,
// devnet=2, regtest=3. `local` (dashmate) maps to regtest. Used for the integer
// `network` field on testRun documents.
export const NETWORK_IDS = {
  mainnet: 0, testnet: 1, devnet: 2, regtest: 3, local: 3,
};
export function networkId(name = getNetwork()) {
  const id = NETWORK_IDS[String(name).toLowerCase()];
  if (id === undefined) throw new Error(`Unknown network '${name}' (expected one of ${Object.keys(NETWORK_IDS).join(', ')}).`);
  return id;
}

// evo-sdk key/address APIs accept a NetworkLike of mainnet/testnet/devnet/regtest
// (not our 'local' alias). Map it so PrivateKey.fromHex / deriveKeyFromSeedWithPath
// work when NETWORK=local (a dashmate regtest node).
export const sdkNetwork = (name = getNetwork()) => (String(name).toLowerCase() === 'local' ? 'regtest' : name);

// Connect a trusted SDK (trusted mode is required so state-transition responses
// are proof-verified). Returns { sdk, mod, network }.
export async function connect() {
  const mod = await loadSdkModule();
  const { EvoSDK } = mod;
  const network = getNetwork();
  let sdk;
  if (network === 'testnet') sdk = EvoSDK.testnetTrusted();
  else if (network === 'mainnet') sdk = EvoSDK.mainnetTrusted();
  else if (network === 'local') sdk = EvoSDK.localTrusted();
  else throw new Error(`Unsupported NETWORK '${network}'. Use testnet, mainnet, or local.`);
  await sdk.connect();
  return { sdk, mod, network };
}

// ---------------------------------------------------------------------------
// Signer + identity key
// ---------------------------------------------------------------------------

// Build a PrivateKey + single-key IdentitySigner from a WIF or 64-char hex key.
export function buildSigner(mod, keyString, network) {
  const { IdentitySigner, PrivateKey } = mod;
  const trimmed = (keyString || '').trim();
  if (!trimmed) throw new Error('Missing private key (set QA_PRIVATE_KEY).');
  const isHex = /^[0-9a-fA-F]{64}$/.test(trimmed);
  const privateKey = isHex
    ? PrivateKey.fromHex(trimmed, sdkNetwork(network))
    : PrivateKey.fromWIF(trimmed);
  const signer = new IdentitySigner();
  signer.addKey(privateKey);
  return { signer, privateKey };
}

function pubKeyHex(privateKey) {
  try {
    const pk = privateKey.getPublicKey();
    if (typeof pk.toHex === 'function') return pk.toHex().toLowerCase();
    if (typeof pk.toString === 'function') return pk.toString().toLowerCase();
  } catch { /* ignore */ }
  return undefined;
}

function normalizeKeyData(data) {
  // IdentityPublicKey.data may surface as a hex string, base64 string, or bytes.
  if (data == null) return undefined;
  if (typeof data === 'string') {
    const s = data.toLowerCase();
    if (/^[0-9a-f]+$/.test(s)) return s; // already hex
    try { return Buffer.from(data, 'base64').toString('hex'); } catch { return s; }
  }
  try { return Buffer.from(data).toString('hex'); } catch { return undefined; }
}

// Resolve the IdentityPublicKey to sign with. If QA_IDENTITY_KEY_ID is set, use
// it directly; otherwise auto-detect the key whose public key matches the
// provided private key. Throws with a helpful key listing if nothing matches.
export function resolveIdentityKey(identity, privateKey) {
  const keys = identity.publicKeys || [];
  const keyIdOf = (k) => Number(k.keyId ?? k.id);
  const explicit = process.env.QA_IDENTITY_KEY_ID;
  if (explicit !== undefined && explicit !== '') {
    const id = Number(explicit);
    const k = typeof identity.getPublicKeyById === 'function'
      ? identity.getPublicKeyById(id)
      : keys.find((x) => keyIdOf(x) === id);
    if (!k) throw new Error(`Identity has no public key with id ${id}.`);
    return k;
  }
  const wantHex = pubKeyHex(privateKey);
  if (wantHex) {
    const match = keys.find((k) => normalizeKeyData(k.data) === wantHex && !k.isReadOnly);
    if (match) return match;
  }
  // Fall back to first writable AUTHENTICATION key (HIGH/CRITICAL) for a clear error if it fails.
  const auth = keys.find((k) => String(k.purpose).toUpperCase() === 'AUTHENTICATION' && !k.isReadOnly);
  if (auth && !wantHex) return auth;
  const listing = keys
    .map((k) => `  id=${keyIdOf(k)} purpose=${k.purpose} security=${k.securityLevel} type=${k.keyType} readOnly=${k.isReadOnly}`)
    .join('\n');
  throw new Error(
    `Could not match the provided private key to any key on identity ${String(identity.id)}.\n`
    + `Set QA_IDENTITY_KEY_ID to pick one explicitly. Identity keys:\n${listing}`,
  );
}

// Load identity + signer + signing key together. Returns { identity, signer, identityKey, privateKey }.
// Every script that signs goes through here, so this is also where a network
// the SDK build cannot follow is refused, before anything is broadcast.
export async function loadOwnerAuth(sdk, mod, network) {
  const ownerId = (process.env.QA_IDENTITY_ID || '').trim();
  if (!ownerId) throw new Error('Missing QA identity (set QA_IDENTITY_ID).');
  await requireKnownProtocolVersion(sdk, mod, network);
  const { signer, privateKey } = buildSigner(mod, process.env.QA_PRIVATE_KEY, network);
  const identity = await sdk.identities.fetch(ownerId);
  if (!identity) throw new Error(`Identity ${ownerId} not found on ${network}.`);
  const identityKey = resolveIdentityKey(identity, privateKey);
  return { ownerId, identity, signer, identityKey, privateKey };
}

// ---------------------------------------------------------------------------
// Admin list
// ---------------------------------------------------------------------------

// The `admin` document naming `identityId`, or undefined. testRun.ownerRefersTo
// reads the same unique index (`byIdentity`), so an identity with a document
// here is one the platform lets record runs. An identifier is compared by its
// base58 string, as in any query on an identifier property.
export async function findAdmin(sdk, contractId, identityId) {
  const res = await sdk.documents.query({
    dataContractId: contractId,
    documentTypeName: 'admin',
    where: [['identity', '==', String(identityId)]],
    limit: 1,
  });
  for (const doc of res.values()) if (doc) return doc;
  return undefined;
}

// Create the `admin` document that lets `identityId` record runs. Only the
// contract owner may sign it.
export async function addAdmin(sdk, mod, {
  contractId, ownerId, identityId, label, signer, identityKey,
}) {
  const { Document, Identifier } = mod;
  const properties = { identity: new Identifier(String(identityId)) };
  if (label) properties.label = label;
  const doc = new Document({
    ownerId,
    dataContractId: contractId,
    documentTypeName: 'admin',
    properties,
    entropy: Uint8Array.from(randomBytes(32)),
  });
  await sdk.documents.create({ document: doc, identityKey, signer });
}

// ---------------------------------------------------------------------------
// Schema + contract-id config
// ---------------------------------------------------------------------------
export function loadSchema() {
  return JSON.parse(readFileSync(SCHEMA_PATH, 'utf8'));
}

export function schemaSha() {
  return createHash('sha256').update(readFileSync(SCHEMA_PATH)).digest('hex').slice(0, 16);
}

export function configPath(network = getNetwork()) {
  return join(QA_DIR, `contract-id.${network}.json`);
}

export function readConfig(network = getNetwork()) {
  const p = configPath(network);
  if (!existsSync(p)) return undefined;
  return JSON.parse(readFileSync(p, 'utf8'));
}

export function writeConfig(cfg, network = getNetwork()) {
  const p = configPath(network);
  writeFileSync(p, `${JSON.stringify(cfg, null, 2)}\n`);
  return p;
}

// Refuse to run against a contract registered with another schema. A contract
// keeps the schema it was registered with, so after a schema change the
// contract-id file still names the old contract until register.mjs --force
// publishes the new one. Scripts that read or write the changed types (admin,
// testRun) would otherwise fail with an unknown type or a missing index.
export function requireCurrentSchema(cfg, network = getNetwork()) {
  const current = schemaSha();
  if (cfg?.schemaSha && cfg.schemaSha !== current) {
    throw new Error(
      `Contract ${cfg.contractId} on ${network} was registered with schema ${cfg.schemaSha}, `
      + `but this checkout's schema is ${current}. The contract owner has to publish the new one `
      + '(node src/register.mjs --force, then seed.mjs, admin.mjs add and migrate-runs.mjs; see '
      + '"Moving to a new contract" in the README) and commit the new contract-id file. '
      + 'To keep using the old contract, check out the commit its schema came from.',
    );
  }
}

// Refuse to write to a network that runs a protocol version this SDK build does
// not know. The write itself can land while the SDK fails to read the proof that
// confirms it: a registration then costs its credits and saves no contract id,
// and a run sent again is recorded twice. Returns the network's protocol version.
//
// `andUpcoming` also refuses while the masternodes are voting for a version the
// SDK does not know. register.mjs asks for it: a registration is paid for once,
// so it must not be sent by an SDK that an upgrade could overtake before the
// confirmation is read.
export async function requireKnownProtocolVersion(sdk, mod, network = getNetwork(), { andUpcoming = false } = {}) {
  const state = await sdk.protocol.versionUpgradeState();
  const plain = state?.toJSON?.() ?? state;
  const current = Number(plain?.currentProtocolVersion);
  const known = Number(await mod.EvoSDK.getLatestVersionNumber());
  if (!Number.isInteger(current) || !Number.isInteger(known)) {
    throw new Error(`Could not compare the protocol version of ${network} (${current}) with the SDK's (${known}).`);
  }
  const useNewerSdk = 'Use an SDK built for that version: set EVO_SDK_BUNDLE to a workspace build '
    + '(see "Install & run" in the README).';
  if (current > known) {
    throw new Error(`${network} runs protocol version ${current}, but this SDK build knows versions up to ${known}. ${useNewerSdk}`);
  }
  const upcoming = plain?.nextProtocolVersion;
  if (andUpcoming && upcoming != null && Number(upcoming) > known) {
    throw new Error(`${network} is voting for protocol version ${Number(upcoming)}, but this SDK build knows versions up to ${known}. `
      + `If it activates before this write is confirmed, the SDK cannot read the confirmation. ${useNewerSdk}`);
  }
  return current;
}
