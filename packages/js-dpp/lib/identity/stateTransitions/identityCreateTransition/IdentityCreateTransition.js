const bs58 = require('bs58');

const hash = require('../../../util/hash');
const AbstractIdentityStateTransition = require('../AbstractIdentityStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identity = require('../../Identity');
const IdentityPublicKey = require('../../IdentityPublicKey');

class IdentityCreateTransition extends AbstractIdentityStateTransition {
  /**
   * @param {RawIdentityCreateTransition} [rawIdentityCreateTransition]
   */
  constructor(rawIdentityCreateTransition) {
    super();

    this.publicKeys = [];

    if (rawIdentityCreateTransition) {
      if (rawIdentityCreateTransition.publicKeys) {
        this.setPublicKeys(
          rawIdentityCreateTransition.publicKeys
            .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
        );
      }

      this
        .setIdentityType(rawIdentityCreateTransition.identityType)
        .setLockedOutPoint(rawIdentityCreateTransition.lockedOutPoint);
    }
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.IDENTITY_CREATE;
  }

  /**
   * Sets an outPoint. OutPoint is a pointer to the output funding identity creation.
   * Its hash also serves as an identity id.
   * More about the OutPoint can be found in the identity documentation
   * @param {string} lockedOutPoint
   * @return {IdentityCreateTransition}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;
    this.identityId = bs58.encode(
      hash(Buffer.from(lockedOutPoint, 'base64')),
    );

    return this;
  }

  /**
   * @return {string}
   */
  getLockedOutPoint() {
    return this.lockedOutPoint;
  }

  /**
   * Sets the identity type.
   * For more info please check identity documentation
   * @param {number} identityType
   * @return {IdentityCreateTransition}
   */
  setIdentityType(identityType) {
    this.identityType = identityType;

    return this;
  }

  /**
   * @return {number}
   */
  getIdentityType() {
    return this.identityType;
  }

  /**
   * @return {IdentityPublicKey[]}
   */
  getPublicKeys() {
    return this.publicKeys;
  }

  /**
   * Replaces existing set of public keys with a new one
   * @param {IdentityPublicKey[]} publicKeys
   * @return {IdentityCreateTransition}
   */
  setPublicKeys(publicKeys) {
    this.publicKeys = publicKeys;

    return this;
  }

  /**
   * Adds public keys to the existing public keys array
   * @param {IdentityPublicKey[]} publicKeys
   * @return {IdentityCreateTransition}
   */
  addPublicKeys(publicKeys) {
    this.publicKeys.push(...publicKeys);

    return this;
  }

  /**
   * Returns base64 representation of the future identity id
   *
   * @return {string}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Get Documents State Transition as plain object
   *
   * @param {Object} [options]
   * @return {RawIdentityCreateTransition}
   */
  toJSON(options) {
    return {
      ...super.toJSON(options),
      identityType: this.getIdentityType(),
      lockedOutPoint: this.getLockedOutPoint(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toJSON()),
    };
  }
}

IdentityCreateTransition.IdentityTypes = Identity.TYPES;

module.exports = IdentityCreateTransition;
