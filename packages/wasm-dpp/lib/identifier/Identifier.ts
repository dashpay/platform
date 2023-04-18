import bs58 from 'bs58';
import IdentifierError from './errors/IdentifierError';

// Buffer extending is not a trivial thing:
// * https://github.com/nodejs/node/commit/651a5b51eb838e8e23a5b94ba34e8e06630a004a
// * https://github.com/nodejs/node/issues/4701
// * https://github.com/nodejs/help/issues/1300
// * https://github.com/nodejs/node/issues/2882

type CborEncoder = {
    pushAny: (buffer: Buffer) => void;
}

type IdentifierEncoding = BufferEncoding | 'base58';

/**
 * @param {Buffer} buffer
 * @returns {Identifier}
 * @constructor
 */
export class Identifier {
  static MEDIA_TYPE = 'application/x.dash.dpp.identifier';

  constructor(buffer: Buffer | Identifier) {
    if (!Buffer.isBuffer(buffer)) {
      throw new IdentifierError('Identifier expects Buffer');
    }

    if (buffer.length !== 32) {
      throw new IdentifierError('Identifier must be 32 long');
    }

    const patchedBuffer = Buffer.from(buffer);

    Object.setPrototypeOf(patchedBuffer, Identifier.prototype);

    // noinspection JSValidateTypes
    // @ts-ignore
    return patchedBuffer;
  }

  /**
   * Convert to Buffer
   *
   * @return {Buffer}
   */
  toBuffer(): Buffer {
    // @ts-ignore
    return Buffer.from(this);
  }

  /**
   * Encode to CBOR
   *
   * @param {CborEncoder} encoder
   * @return {boolean}
   */
  encodeCBOR(encoder: CborEncoder): boolean {
    encoder.pushAny(this.toBuffer());

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
  toString(encoding: IdentifierEncoding = 'base58'): string {
    if (encoding === 'base58') {
      return bs58.encode(this.toBuffer());
    }

    return this.toBuffer().toString(encoding);
  }

  /**
   * Compare to another Identifier
   * @param other
   */
  equals(other: Identifier | Buffer): boolean {
    // @ts-ignore
    return this.toBuffer().equals(Buffer.from(other));
  }

  /**
   * Create Identifier from buffer or encoded string
   *
   * @param {string|Buffer|Identifier} value
   * @param {string} encoding
   * @return {Identifier}
   */
  static from(value: string | Buffer | Identifier, encoding: string = undefined): Identifier {
    let buffer;

    if (typeof value === 'string') {
      if (encoding === undefined) {
        // eslint-disable-next-line no-param-reassign
        encoding = 'base58';
      }

      if (encoding === 'base58') {
        buffer = bs58.decode(value);
      } else {
        buffer = Buffer.from(value, 'base64');
      }
    } else {
      if (encoding !== undefined) {
        throw new IdentifierError('encoding accepted only with type string');
      }

      buffer = value;
    }

    return new Identifier(buffer);
  }
}

Object.setPrototypeOf(Identifier.prototype, Buffer.prototype);

export default Identifier;
