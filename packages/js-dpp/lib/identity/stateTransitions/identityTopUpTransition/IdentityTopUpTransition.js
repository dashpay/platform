const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identifier = require('../../../Identifier');

class IdentityTopUpTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityTopUpTransition} [rawIdentityTopUpTransition]
   */
  constructor(rawIdentityTopUpTransition = {}) {
    super(rawIdentityTopUpTransition);

    if (Object.prototype.hasOwnProperty.call(rawIdentityTopUpTransition, 'lockedOutPoint')) {
      this.setLockedOutPoint(rawIdentityTopUpTransition.lockedOutPoint);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityTopUpTransition, 'identityId')) {
      this.setIdentityId(rawIdentityTopUpTransition.identityId);
    }
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.IDENTITY_TOP_UP;
  }

  /**
   * Sets an outPoint. OutPoint is a pointer to the output funding the top up.
   * More about the OutPoint can be found in the identity documentation
   *
   * @param {Buffer} lockedOutPoint
   * @return {IdentityTopUpTransition}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;

    return this;
  }

  /**
   * @return {Buffer}
   */
  getLockedOutPoint() {
    return this.lockedOutPoint;
  }

  /**
   * Returns base58 representation of the identity id top up
   *
   * @param {Buffer} identityId
   * @return {IdentityTopUpTransition}
   */
  setIdentityId(identityId) {
    this.identityId = Identifier.from(identityId);

    return this;
  }

  /**
   * Returns identity id
   *
   * @return {Identifier}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Returns Owner ID
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.identityId;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawIdentityTopUpTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawStateTransition = {
      ...super.toObject(options),
      identityId: this.getIdentityId(),
      lockedOutPoint: this.getLockedOutPoint(),
    };

    if (!options.skipIdentifiersConversion) {
      rawStateTransition.identityId = this.getIdentityId().toBuffer();
    }

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonIdentityTopUpTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      identityId: this.getIdentityId().toString(),
      lockedOutPoint: this.getLockedOutPoint().toString('base64'),
    };
  }
}

/**
 * @typedef {RawStateTransition & Object} RawIdentityTopUpTransition
 * @property {Buffer} lockedOutPoint
 * @property {Buffer} identityId
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityTopUpTransition
 * @property {string} lockedOutPoint
 * @property {string} identityId
 */

module.exports = IdentityTopUpTransition;
