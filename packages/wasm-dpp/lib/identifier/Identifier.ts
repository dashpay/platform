import bs58 from 'bs58';
import IdentifierError from './errors/IdentifierError';

// Buffer extending is not a trivial thing — Identifier used to be a Buffer
// subclass. As of this revision, Identifier extends Uint8Array directly so it
// can be constructed and used in environments without a Buffer polyfill.
// The Buffer-returning toBuffer() method is retained as a deprecated shim
// for backward compatibility; it requires Buffer to exist at call time but
// no longer at module load.
// * https://github.com/nodejs/node/commit/651a5b51eb838e8e23a5b94ba34e8e06630a004a
// * https://github.com/nodejs/node/issues/4701
// * https://github.com/nodejs/help/issues/1300
// * https://github.com/nodejs/node/issues/2882

type CborEncoder = {
    pushAny: (bytes: Uint8Array) => void;
}

// Encodings accepted by toString(). 'hex' decoding is not supported by from()
// because there is no precedent for it on the public surface and bs58/base64
// already cover the user-facing identifier formats.
type IdentifierStringEncoding = 'base58' | 'base64' | 'hex';
type IdentifierFromEncoding = 'base58' | 'base64';

function bytesToHex(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i += 1) {
    out += bytes[i].toString(16).padStart(2, '0');
  }
  return out;
}

function bytesToBase64(bytes: Uint8Array): string {
  let bin = '';
  for (let i = 0; i < bytes.length; i += 1) {
    bin += String.fromCharCode(bytes[i]);
  }
  return btoa(bin);
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

/**
 * @param {Uint8Array} bytes
 * @returns {Identifier}
 * @constructor
 */
export class Identifier {
  static MEDIA_TYPE = 'application/x.dash.dpp.identifier';

  constructor(bytes: Uint8Array | Identifier) {
    // Accept both Node Buffer and plain Uint8Array. Buffer extends Uint8Array,
    // so an instanceof Uint8Array check matches both — existing Buffer callers
    // continue to work unchanged.
    if (!(bytes instanceof Uint8Array)) {
      throw new IdentifierError('Identifier expects Uint8Array');
    }

    if (bytes.length !== 32) {
      throw new IdentifierError('Identifier must be 32 long');
    }

    // Copy into a fresh Uint8Array so callers cannot mutate the underlying
    // bytes after construction, then patch the prototype so the result is
    // an Identifier instance.
    const patched = new Uint8Array(bytes);

    Object.setPrototypeOf(patched, Identifier.prototype);

    // noinspection JSValidateTypes
    // @ts-ignore
    return patched;
  }

  /**
   * Convert to a plain Uint8Array (copy)
   *
   * @return {Uint8Array}
   */
  toBytes(): Uint8Array {
    return new Uint8Array(this as unknown as Uint8Array);
  }

  /**
   * Convert to Buffer
   *
   * @deprecated Prefer toBytes(). This method requires the `Buffer` global
   *   to exist at call time and is retained only for backward compatibility.
   * @return {Buffer}
   */
  toBuffer(): Buffer {
    return Buffer.from(this as unknown as Uint8Array);
  }

  /**
   * Encode to CBOR
   *
   * @param {CborEncoder} encoder
   * @return {boolean}
   */
  encodeCBOR(encoder: CborEncoder): boolean {
    encoder.pushAny(this.toBytes());

    return true;
  }

  /**
   * Convert to JSON
   *
   * @return {string}
   */
  toJSON(): string {
    return this.toString();
  }

  /**
   * Encode to string
   *
   * @param {string} [encoding=base58]
   * @return {string}
   */
  toString(encoding: IdentifierStringEncoding = 'base58'): string {
    const bytes = this.toBytes();
    if (encoding === 'base58') {
      return bs58.encode(bytes);
    }
    if (encoding === 'base64') {
      return bytesToBase64(bytes);
    }
    if (encoding === 'hex') {
      return bytesToHex(bytes);
    }
    throw new IdentifierError(`Unsupported encoding: ${encoding}`);
  }

  /**
   * Compare to another Identifier or Uint8Array
   *
   * @param {Identifier | Uint8Array} other
   */
  equals(other: Identifier | Uint8Array): boolean {
    if (!(other instanceof Uint8Array)) {
      return false;
    }
    if (other.length !== this.length) {
      return false;
    }
    for (let i = 0; i < this.length; i += 1) {
      // @ts-ignore — Identifier is a Uint8Array at runtime
      if (this[i] !== other[i]) {
        return false;
      }
    }
    return true;
  }

  /**
   * Create Identifier from bytes or encoded string
   *
   * @param {string | Uint8Array | Identifier} value
   * @param {string} [encoding] — when value is a string, one of 'base58' (default) or 'base64'
   * @return {Identifier}
   */
  static from(value: string | Uint8Array | Identifier, encoding?: IdentifierFromEncoding): Identifier {
    let bytes;

    if (typeof value === 'string') {
      const effectiveEncoding = encoding === undefined ? 'base58' : encoding;

      if (effectiveEncoding === 'base58') {
        // bs58.decode returns a Uint8Array (currently backed by Buffer via
        // safe-buffer, which transitively requires a Buffer polyfill in
        // browsers; once bs58 is replaced with a Uint8Array-native base58
        // library, that transitive dependency goes away).
        bytes = bs58.decode(value);
      } else if (effectiveEncoding === 'base64') {
        bytes = base64ToBytes(value);
      } else {
        throw new IdentifierError(`Unsupported encoding for string input: ${effectiveEncoding}`);
      }
    } else {
      if (encoding !== undefined) {
        throw new IdentifierError('encoding accepted only with type string');
      }

      bytes = value;
    }

    return new Identifier(bytes);
  }

  // Required to satisfy TypeScript's structural typing when this class is
  // declared as `extends _Identifier` elsewhere; instances are real Uint8Arrays
  // at runtime via the prototype patch above.
  declare length: number;
}

Object.setPrototypeOf(Identifier.prototype, Uint8Array.prototype);

export default Identifier;
