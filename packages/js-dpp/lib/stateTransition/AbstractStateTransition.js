const hash = require('../util/hash');
const { encode } = require('../util/serializer');

/**
 * @abstract
 */
class AbstractStateTransition {
  /**
   * Get protocol version
   *
   * @return {number}
   */
  getProtocolVersion() {
    return 0;
  }

  /**
   * @abstract
   */
  getType() {
    throw new Error('Not implemented');
  }

  /**
   * @abstract
   * @return {{protocolVersion: number, type: number}}
   */
  toJSON() {
    return {
      protocolVersion: this.getProtocolVersion(),
      type: this.getType(),
    };
  }

  /**
   * Return serialized State Transition
   *
   * @return {Buffer}
   */
  serialize() {
    return encode(this.toJSON());
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }
}

module.exports = AbstractStateTransition;
