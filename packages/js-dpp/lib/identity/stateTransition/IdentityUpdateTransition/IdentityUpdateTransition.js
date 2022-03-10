const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identifier = require('../../../identifier/Identifier');
const IdentityPublicKey = require('../../IdentityPublicKey');

class IdentityUpdateTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityUpdateTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(rawStateTransition);

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'identityId')) {
      this.setIdentityId(rawStateTransition.identityId);
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'revision')) {
      this.setRevision(rawStateTransition.revision);
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'addPublicKeys')) {
      this.setAddPublicKeys(
        rawStateTransition.addPublicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'disablePublicKeys')) {
      this.setDisablePublicKeys(
        rawStateTransition.disablePublicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'publicKeysDisabledAt')) {
      this.setPublicKeysDisabledAt(rawStateTransition.publicKeysDisabledAt);
    }
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.IDENTITY_UPDATE;
  }

  /**
   * Returns base58 representation of the identity id top up
   *
   * @param {Buffer} identityId
   * @return {IdentityUpdateTransition}
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
   * Get revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
  }

  /**
   * Set revision
   *
   * @param {number} revision
   * @return {IdentityUpdateTransition}
   */
  setRevision(revision) {
    this.revision = revision;

    return this;
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
   * Get public keys to add to the Identity.
   *
   * @returns {IdentityPublicKey[]}
   */
  getAddPublicKeys() {
    return this.addPublicKeys;
  }

  /**
   * Set public keys to add to the Identity.
   *
   * @param {IdentityPublicKey[]} publicKeys
   * @returns {IdentityUpdateTransition}
   */
  setAddPublicKeys(publicKeys) {
    this.addPublicKeys = publicKeys;

    return this;
  }

  /**
   *
   * Get Identity Public key IDs to disable for the Identity.
   *
   * @returns {number[]}
   */
  getDisablePublicKeys() {
    return this.disablePublicKeys;
  }

  /**
   *
   * Set Identity Public key IDs to disable for the Identity.
   *
   * @param {number[]} publicKeys
   * @returns {IdentityUpdateTransition}
   */
  setDisablePublicKeys(publicKeyIds) {
    this.disablePublicKeys = publicKeyIds;

    return this;
  }

  /**
   * Get timestamp when keys were disabled.
   *
   * @returns {number}
   */
  getPublicKeysDisabledAt() {
    return this.publicKeysDisabledAt;
  }

  /**
   * Set timestamp when keys were disabled.
   *
   * @param {number} publicKeysDisabledAt
   * @returns {IdentityUpdateTransition}
   */
  setPublicKeysDisabledAt(publicKeysDisabledAt) {
    this.publicKeysDisabledAt = publicKeysDisabledAt;

    return this;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawIdentityUpdateTransition}
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
      revision: this.getRevision(),
    };

    if (this.getPublicKeysDisabledAt()) {
      rawStateTransition.publicKeysDisabledAt = this.getPublicKeysDisabledAt();
    }

    if (this.getAddPublicKeys()) {
      rawStateTransition.addPublicKeys = this.getAddPublicKeys()
        .map((publicKey) => publicKey.toObject());
    }

    if (this.getDisablePublicKeys()) {
      rawStateTransition.disablePublicKeys = this.getDisablePublicKeys();
    }

    if (!options.skipIdentifiersConversion) {
      rawStateTransition.identityId = this.getIdentityId().toBuffer();
    }

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonIdentityUpdateTransition}
   */
  toJSON() {
    const jsonStateTransition = {
      ...super.toJSON(),
      identityId: this.getIdentityId().toString(),
      revision: this.getRevision(),
    };

    if (this.getPublicKeysDisabledAt()) {
      jsonStateTransition.publicKeysDisabledAt = this.getPublicKeysDisabledAt();
    }

    if (this.getAddPublicKeys()) {
      jsonStateTransition.addPublicKeys = this.getAddPublicKeys()
        .map((publicKey) => publicKey.toJSON());
    }

    if (this.getDisablePublicKeys()) {
      jsonStateTransition.disablePublicKeys = this.getDisablePublicKeys();
    }

    return jsonStateTransition;
  }

  /**
   * Returns ids of topped up identities
   *
   * @return {Identifier[]}
   */
  getModifiedDataIds() {
    return [this.getIdentityId()];
  }
}

/**
 * @typedef {RawStateTransition & Object} RawIdentityUpdateTransition
 * @property {Buffer} identityId
 * @property {number} revision
 * @property {IdentityPublicKey[]} [addPublicKeys]
 * @property {number[]} [disablePublicKeys]
 * @property {number} [publicKeysDisabledAt]
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityUpdateTransition
 * @property {Buffer} identityId
 * @property {number} revision
 * @property {IdentityPublicKey[]} [addPublicKeys]
 * @property {number[]} [disablePublicKeys]
 * @property {number} [publicKeysDisabledAt]
 */

module.exports = IdentityUpdateTransition;
