// Lightweight byte utilities used in place of Node Buffer methods on the
// public surface. Inputs that look like bytes are required to be Uint8Array
// (Buffer extends Uint8Array, so existing Buffer callers continue to work).

function assertBytes(value, name) {
  if (!(value instanceof Uint8Array)) {
    throw new TypeError(`${name} must be a Uint8Array`);
  }
}

export function hexToBytes(hex) {
  if (typeof hex !== 'string') {
    throw new TypeError('hex must be a string');
  }
  if (hex.length % 2 !== 0) {
    throw new Error('hex must have even length');
  }
  if (/[^0-9a-fA-F]/.test(hex)) {
    throw new Error('hex contains non-hex characters');
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

export function bytesToHex(bytes) {
  assertBytes(bytes, 'bytes');
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export function base64ToBytes(b64) {
  if (typeof b64 !== 'string') {
    throw new TypeError('b64 must be a string');
  }
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

export function bytesToBase64(bytes) {
  assertBytes(bytes, 'bytes');
  let bin = '';
  for (let i = 0; i < bytes.length; i += 1) {
    bin += String.fromCharCode(bytes[i]);
  }
  return btoa(bin);
}

export function concatBytes(arrays) {
  if (!Array.isArray(arrays)) {
    throw new TypeError('concatBytes expects an array');
  }
  arrays.forEach((a, i) => assertBytes(a, `arrays[${i}]`));
  let total = 0;
  for (const a of arrays) total += a.length;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const a of arrays) {
    out.set(a, offset);
    offset += a.length;
  }
  return out;
}

export function bytesEqual(a, b) {
  assertBytes(a, 'a');
  assertBytes(b, 'b');
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}
