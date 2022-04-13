const {
  PublicKey,
  PrivateKey,
  crypto: { Hash },
} = require('@dashevo/dashcore-lib');

const AbstractStateTransition = require('./AbstractStateTransition');

const IdentityPublicKey = require('../identity/IdentityPublicKey');
const InvalidSignaturePublicKeyError = require('./errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('./errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('./errors/PublicKeyMismatchError');
const PublicKeySecurityLevelNotMetError = require('./errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('./errors/WrongPublicKeyPurposeError');
const InvalidIdentityPublicKeyTypeError = require('./errors/InvalidIdentityPublicKeyTypeError');
const blsPrivateKeyFactory = require('../bls/blsPrivateKeyFactory');
const blsPublicKeyFactory = require('../bls/blsPublicKeyFactory');

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
   * @return {Promise<AbstractStateTransition>}
   */
  async sign(identityPublicKey, privateKey) {
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

        await this.signByPrivateKey(privateKeyModel, identityPublicKey.getType());
        break;
      case IdentityPublicKey.TYPES.ECDSA_HASH160: {
        privateKeyModel = new PrivateKey(privateKey);
        pubKeyBase = new PublicKey({
          ...privateKeyModel.toPublicKey().toObject(),
          compressed: true,
        })
          .toBuffer();

        pubKeyBase = Hash.sha256ripemd160(pubKeyBase);

        if (!pubKeyBase.equals(identityPublicKey.getData())) {
          throw new InvalidSignaturePublicKeyError(identityPublicKey.getData());
        }

        await this.signByPrivateKey(privateKeyModel, identityPublicKey.getType());
        break;
      }
      case IdentityPublicKey.TYPES.BLS12_381:
        privateKeyModel = await blsPrivateKeyFactory(privateKey);
        pubKeyBase = Buffer.from(privateKeyModel.getPublicKey().serialize());

        if (!pubKeyBase.equals(identityPublicKey.getData())) {
          throw new InvalidSignaturePublicKeyError(identityPublicKey.getData());
        }

        await this.signByPrivateKey(privateKeyModel, identityPublicKey.getType());
        break;
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
        publicKey.getSecurityLevel(),
        this.getKeySecurityLevelRequirement(),
      );
    }

    if (publicKey.getPurpose() !== IdentityPublicKey.PURPOSES.AUTHENTICATION) {
      throw new WrongPublicKeyPurposeError(
        publicKey.getPurpose(),
        IdentityPublicKey.PURPOSES.AUTHENTICATION,
      );
    }
  }

  /**
   * Verify signature
   *
   * @param {IdentityPublicKey} publicKey
   * @return {Promise<boolean>}
   */
  async verifySignature(publicKey) {
    this.verifyPublicKeyLevelAndPurpose(publicKey);

    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    if (this.getSignaturePublicKeyId() !== publicKey.getId()) {
      throw new PublicKeyMismatchError(publicKey);
    }

    const publicKeyBuffer = publicKey.getData();

    switch (publicKey.getType()) {
      case IdentityPublicKey.TYPES.ECDSA_HASH160:
        return this.verifyESDSAHash160SignatureByPublicKeyHash(publicKeyBuffer);
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1:
        return this.verifyECDSASignatureByPublicKey(PublicKey.fromBuffer(publicKeyBuffer));
      case IdentityPublicKey.TYPES.BLS12_381: {
        const publicKeyModel = await blsPublicKeyFactory(new Uint8Array(publicKeyBuffer));

        return this.verifyBLSSignatureByPublicKey(publicKeyModel);
      }
      default:
        throw new InvalidIdentityPublicKeyTypeError(publicKey.getType());
    }
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
