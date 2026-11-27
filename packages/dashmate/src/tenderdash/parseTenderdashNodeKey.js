import crypto from 'node:crypto';
import deriveTenderdashNodeId from './deriveTenderdashNodeId.js';

const NODE_KEY_LENGTH = 64;

const ED25519_KEY_TYPE = 'tendermint/PrivKeyEd25519';

// Standard base64 with its padding: the form Tenderdash writes and dashmate
// stores. Buffer.from() would skip anything else instead of failing.
const BASE64_PATTERN = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;

const PKCS8_ED25519_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex');
const SPKI_ED25519_PREFIX = Buffer.from('302a300506032b6570032100', 'hex');

/**
 * Decode the Tenderdash base64 form and check that its public half belongs to
 * its private half
 *
 * @param {string} value
 * @return {string} the key, re-encoded
 */
function decodeKey(value) {
  if (!BASE64_PATTERN.test(value)) {
    throw new Error('The node key is not valid base64');
  }

  const nodeKey = Buffer.from(value, 'base64');

  if (nodeKey.length !== NODE_KEY_LENGTH) {
    throw new Error(`The node key must be ${NODE_KEY_LENGTH} bytes: an Ed25519 private key`
      + ' followed by its public key');
  }

  const derivedPublicKey = crypto.createPublicKey({
    format: 'der',
    type: 'pkcs8',
    key: Buffer.concat([PKCS8_ED25519_PREFIX, nodeKey.subarray(0, 32)]),
  }).export({ format: 'der', type: 'spki' });

  if (!derivedPublicKey.equals(Buffer.concat([SPKI_ED25519_PREFIX, nodeKey.subarray(32)]))) {
    throw new Error('The public key in the node key does not belong to its private key');
  }

  return nodeKey.toString('base64');
}

/**
 * Read a Tenderdash node key from what an operator supplies
 *
 * Two forms are accepted: the base64 string dashmate stores in
 * `platform.drive.tenderdash.node.key`, and a Tenderdash `node_key.json`, which
 * is also what `tenderdash gen-node-key` prints. Surrounding whitespace is
 * ignored, so a backup written with `dashmate config get … > file` reads back.
 *
 * Refusals are fixed texts. The input is a secret, so no message may quote it:
 * Node's JSON.parse errors do, which is why they are not passed on.
 *
 * @typedef {function} parseTenderdashNodeKey
 * @param {string} input
 * @return {{ key: string, id: string }}
 */
export default function parseTenderdashNodeKey(input) {
  const text = String(input).trim();

  if (text.length === 0) {
    throw new Error('The node key is empty');
  }

  if (!text.startsWith('{')) {
    const key = decodeKey(text);

    return { key, id: deriveTenderdashNodeId(key) };
  }

  let file;
  try {
    file = JSON.parse(text);
  } catch {
    throw new Error('The node key file is not valid JSON');
  }

  const privateKey = file?.priv_key;

  if (typeof privateKey?.value !== 'string') {
    throw new Error('The node key file is not a Tenderdash node_key.json: priv_key.value is missing');
  }

  if (privateKey.type !== ED25519_KEY_TYPE) {
    throw new Error(`The node key file must hold an Ed25519 key (priv_key.type ${ED25519_KEY_TYPE})`);
  }

  const key = decodeKey(privateKey.value);
  const id = deriveTenderdashNodeId(key);

  if (file.id !== undefined && String(file.id).toLowerCase() !== id) {
    throw new Error('The id in the node key file does not match its key');
  }

  return { key, id };
}
