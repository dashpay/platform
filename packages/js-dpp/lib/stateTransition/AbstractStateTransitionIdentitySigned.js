const {
  PublicKey,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const AbstractStateTransition = require('./AbstractStateTransition');

const IdentityPublicKey = require('../identity/IdentityPublicKey');
const InvalidSignaturePublicKeyError = require('./errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('./errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('./errors/PublicKeyMismatchError');
const PublicKeySecurityLevelNotMetError = require('./errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('./errors/WrongPublicKeyPurposeError');
const InvalidIdentityPublicKeyTypeError = require('./errors/InvalidIdentityPublicKeyTypeError');

/**
 * @abstract
 */
class AbstractStateTransitionIdentitySigned extends AbstractStateTransition {
  /**
   * @param {
   * RawDataContractCreateTransition|
   * RawDocumentsBatchTransition
   * } [rawStateTransition]
   */
  constructor(rawStateTransition = {}) {
    super(rawStateTransition);

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'signaturePublicKeyId')) {
      this.signaturePublicKeyId = rawStateTransition.signaturePublicKeyId;
    }
  }

  /**
   * Returns public key id
   *
   * @returns {number}
   */
  getSignaturePublicKeyId() {
    return this.signaturePublicKeyId;
  }

  /**
   * Sign data and check identityPublicKey
   *
   * @param {IdentityPublicKey} identityPublicKey
   * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex or base58
   * @return {AbstractStateTransition}
   */
  sign(identityPublicKey, privateKey) {
    let privateKeyModel;
    let pubKeyBase;

    this.verifyPublicKeyLevelAndPurpose(identityPublicKey);

    switch (identityPublicKey.getType()) {
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1:
        privateKeyModel = new PrivateKey(privateKey);

        /* We store compressed public key in the identity as a base64 string,
        /* and here we compare the private key used to sign the state transition
        /* with the compressed key stored in the identity */
        pubKeyBase = new PublicKey({
          ...privateKeyModel.toPublicKey().toObject(),
          compressed: true,
        })
          .toBuffer();

        if (!pubKeyBase.equals(identityPublicKey.getData())) {
          throw new InvalidSignaturePublicKeyError(identityPublicKey.getData());
        }

        this.signByPrivateKey(privateKeyModel);
        break;
      case IdentityPublicKey.TYPES.BLS12_381:
      default:
        throw new InvalidIdentityPublicKeyTypeError(identityPublicKey.getType());
    }

    this.signaturePublicKeyId = identityPublicKey.getId();

    return this;
  }

  /**
   * @private
   *
   * Verifies that the supplied public key has the correct security level
   * and purpose to sign this state transition
   */
  verifyPublicKeyLevelAndPurpose(publicKey) {
    if (this.getKeySecurityLevelRequirement() < publicKey.getSecurityLevel()) {
      throw new PublicKeySecurityLevelNotMetError(
        publicKey,
        this.getKeySecurityLevelRequirement(),
      );
    }

    if (publicKey.getPurpose() !== IdentityPublicKey.PURPOSES.AUTHENTICATION) {
      throw new WrongPublicKeyPurposeError(
        publicKey,
        IdentityPublicKey.PURPOSES.AUTHENTICATION,
      );
    }
  }

  /**
   * Verify signature
   *
   * @param {IdentityPublicKey} publicKey
   * @return {boolean}
   */
  verifySignature(publicKey) {
    this.verifyPublicKeyLevelAndPurpose(publicKey);

    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    if (this.getSignaturePublicKeyId() !== publicKey.getId()) {
      throw new PublicKeyMismatchError(publicKey);
    }

    const publicKeyBuffer = publicKey.getData();
    const publicKeyModel = PublicKey.fromBuffer(publicKeyBuffer);

    return this.verifySignatureByPublicKey(publicKeyModel);
  }

  /**
   * Set signature public key id
   * @param {number} signaturePublicKeyId
   * @return {AbstractStateTransition}
   */
  setSignaturePublicKeyId(signaturePublicKeyId) {
    this.signaturePublicKeyId = signaturePublicKeyId;

    return this;
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
    const skipSignature = !!options.skipSignature;

    const rawStateTransition = super.toObject(options);

    if (!skipSignature) {
      rawStateTransition.signaturePublicKeyId = this.getSignaturePublicKeyId();
    }

    return rawStateTransition;
  }

  /**
   * Returns minimal key security level that can be used to sign this ST.
   * Override this method if the ST requires a different security level.
   *
   * @return {number}
   */
  getKeySecurityLevelRequirement() {
    return IdentityPublicKey.SECURITY_LEVELS.MASTER;
  }
}

/**
 * @typedef {RawStateTransition & Object} RawStateTransitionIdentitySigned
 * @property {Buffer} [signaturePublicKeyId]
 */

/**
 * @typedef {JsonStateTransition & Object} JsonStateTransitionIdentitySigned
 * @property {string} [signaturePublicKeyId]
 */

module.exports = AbstractStateTransitionIdentitySigned;
