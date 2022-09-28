const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identifier = require('../../../identifier/Identifier');
const IdentityPublicKey = require('../../IdentityPublicKey');
const AbstractStateTransitionIdentitySigned = require('../../../stateTransition/AbstractStateTransitionIdentitySigned');

class IdentityUpdateTransition extends AbstractStateTransitionIdentitySigned {
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
      this.setPublicKeysToAdd(
        rawStateTransition.addPublicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'disablePublicKeys')) {
      this.setPublicKeyIdsToDisable(
        rawStateTransition.disablePublicKeys,
      );
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'publicKeysDisabledAt')) {
      this.setPublicKeysDisabledAt(new Date(rawStateTransition.publicKeysDisabledAt));
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
  getPublicKeysToAdd() {
    return this.addPublicKeys;
  }

  /**
   * Set public keys to add to the Identity.
   *
   * @param {IdentityPublicKey[]} publicKeys
   * @returns {IdentityUpdateTransition}
   */
  setPublicKeysToAdd(publicKeys) {
    this.addPublicKeys = publicKeys;

    return this;
  }

  /**
   *
   * Get Identity Public key IDs to disable for the Identity.
   *
   * @returns {number[]}
   */
  getPublicKeyIdsToDisable() {
    return this.disablePublicKeys;
  }

  /**
   *
   * Set Identity Public key IDs to disable for the Identity.
   *
   * @param {number[]} publicKeyIds
   * @returns {IdentityUpdateTransition}
   */
  setPublicKeyIdsToDisable(publicKeyIds) {
    this.disablePublicKeys = publicKeyIds;

    return this;
  }

  /**
   * Get timestamp when keys were disabled.
   *
   * @returns {Date}
   */
  getPublicKeysDisabledAt() {
    return this.publicKeysDisabledAt;
  }

  /**
   * Set timestamp when keys were disabled.
   *
   * @param {Date} publicKeysDisabledAt
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
      rawStateTransition.publicKeysDisabledAt = this.getPublicKeysDisabledAt().getTime();
    }

    if (this.getPublicKeysToAdd()) {
      rawStateTransition.addPublicKeys = this.getPublicKeysToAdd()
        .map((publicKey) => publicKey.toObject(options));
    }

    if (this.getPublicKeyIdsToDisable()) {
      rawStateTransition.disablePublicKeys = this.getPublicKeyIdsToDisable();
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
      jsonStateTransition.publicKeysDisabledAt = this.getPublicKeysDisabledAt().getTime();
    }

    if (this.getPublicKeysToAdd()) {
      jsonStateTransition.addPublicKeys = this.getPublicKeysToAdd()
        .map((publicKey) => publicKey.toJSON());
    }

    if (this.getPublicKeyIdsToDisable()) {
      jsonStateTransition.disablePublicKeys = this.getPublicKeyIdsToDisable();
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

  /**
   * Returns minimal key security level that can be used to sign this ST
   *
   * @override
   * @return {number}
   */
  getKeySecurityLevelRequirement() {
    return IdentityPublicKey.SECURITY_LEVELS.MASTER;
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
