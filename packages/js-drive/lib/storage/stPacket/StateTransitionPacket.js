const multihashes = require('multihashes');
const CID = require('cids');
const doubleSha256 = require('../../util/doubleSha256');

const InvalidHashError = require('../errors/InvalidHashError');

const PACKET_FIELDS = ['pver', 'dapid', 'dapobjectshash', 'dapcontract', 'dapobjects', 'meta'];

class StateTransitionPacket {
  constructor(data) {
    Object.assign(this, data);
  }

  /**
   * Get packet IPFS CID
   *
   * @return {CID}
   */
  getCID() {
    return StateTransitionPacket.createCIDFromHash(this.getHash());
  }

  /**
   * Get packet hash
   *
   * @return {string}
   */
  getHash() {
    const data = this.toJSON({ skipMeta: true });
    return doubleSha256(data);
  }

  /**
   * @param [skipMeta]
   */
  toJSON({ skipMeta = false }) {
    const result = {};
    PACKET_FIELDS.forEach((field) => {
      if (this[field] !== undefined) {
        result[field] = this[field];
      }
    });

    if (skipMeta) {
      delete result.meta;
    }

    return result;
  }

  /**
   * Create IPFS CID from hash
   *
   * @throws InvalidMultihashError
   * @param {string} hash
   * @return {CID}
   */
  static createCIDFromHash(hash) {
    const buffer = Buffer.from(hash, 'hex');
    const multihash = multihashes.encode(buffer, 'dbl-sha2-256');
    try {
      return new CID(1, 'dag-cbor', multihash);
    } catch (e) {
      throw new InvalidHashError(`could not create CID: ${e.message}`);
    }
  }
}

module.exports = StateTransitionPacket;
