const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');

class IdentityTopUpTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityTopUpTransition} [rawIdentityTopUpTransition]
   */
  constructor(rawIdentityTopUpTransition) {
    super(rawIdentityTopUpTransition);

    if (rawIdentityTopUpTransition) {
      this
        .setLockedOutPoint(rawIdentityTopUpTransition.lockedOutPoint)
        .setIdentityId(rawIdentityTopUpTransition.identityId);
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
   * @param {string} lockedOutPoint
   * @return {IdentityTopUpTransition}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;

    return this;
  }

  /**
   * @return {string}
   */
  getLockedOutPoint() {
    return this.lockedOutPoint;
  }

  /**
   * Returns base58 representation of the identity id top up
   *
   * @param {string} identityId
   * @return {IdentityTopUpTransition}
   */
  setIdentityId(identityId) {
    this.identityId = identityId;

    return this;
  }

  /**
   * Returns base58 representation of the identity id top up
   *
   * @return {string}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Returns Owner ID
   *
   * @return {string}
   */
  getOwnerId() {
    return this.identityId;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature]
   *
   * @return {Object}
   */
  toObject(options = {}) {
    return {
      ...super.toObject(options),
      identityId: this.getIdentityId(),
      lockedOutPoint: this.getLockedOutPoint(),
    };
  }

  /**
   * Create state transition from JSON
   *
   * @param {RawIdentityTopUpTransition} rawStateTransition
   *
   * @return {IdentityTopUpTransition}
   */
  static fromJSON(rawStateTransition) {
    return new IdentityTopUpTransition(
      AbstractStateTransition.translateJsonToObject(rawStateTransition),
    );
  }
}

/**
 * @typedef {Object} RawIdentityTopUpTransition
 * @extends RawStateTransition
 * @property {string} lockedOutPoint
 * @property {string} identityId
 */

module.exports = IdentityTopUpTransition;
