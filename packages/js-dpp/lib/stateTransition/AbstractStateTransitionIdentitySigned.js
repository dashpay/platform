const {
  PublicKey,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const AbstractStateTransition = require('./AbstractStateTransition');

const IdentityPublicKey = require('../identity/IdentityPublicKey');
const InvalidSignatureTypeError = require('./errors/InvalidSignatureTypeError');
const InvalidSignaturePublicKeyError = require('./errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('./errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('./errors/PublicKeyMismatchError');
const InvalidIdentityPublicKeyTypeError = require('../errors/InvalidIdentityPublicKeyTypeError');
const encodeToBase64WithoutPadding = require('../util/encoding/encodeToBase64WithoutPadding');

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
   * @returns {number|null}
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

        if (encodeToBase64WithoutPadding(pubKeyBase) !== identityPublicKey.getData().toString()) {
          throw new InvalidSignaturePublicKeyError(identityPublicKey.getData());
        }

        this.signByPrivateKey(privateKeyModel);
        break;
      case IdentityPublicKey.TYPES.BLS12_381:
        throw new InvalidIdentityPublicKeyTypeError(identityPublicKey.getType());
      default:
        throw new InvalidSignatureTypeError(identityPublicKey.getType());
    }

    this.signaturePublicKeyId = identityPublicKey.getId();

    return this;
  }

  /**
   * Verify signature
   *
   * @param {IdentityPublicKey} publicKey
   * @return {boolean}
   */
  verifySignature(publicKey) {
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
