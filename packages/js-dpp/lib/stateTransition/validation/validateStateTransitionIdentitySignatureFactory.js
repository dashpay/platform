const IdentityPublicKey = require('../../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeConsensusError = require('../../errors/consensus/signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureConsensusError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyConsensusError = require('../../errors/consensus/signature/MissingPublicKeyError');
const StateTransitionIsNotSignedConsensusError = require('../../errors/consensus/signature/StateTransitionIsNotSignedError');
const PublicKeyMismatchConsensusError = require('../../errors/consensus/signature/PublicKeyMismatchError');
const InvalidSignaturePublicKeySecurityLevelConsensusError = require('../../errors/consensus/signature/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeySecurityLevelNotMetConsensusError = require('../../errors/consensus/signature/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeConsensusError = require('../../errors/consensus/signature/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledConsensusError = require('../../errors/consensus/signature/PublicKeyIsDisabledError');
const DPPError = require('../../errors/DPPError');
const StateTransitionIsNotSignedError = require('../errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('../errors/PublicKeyMismatchError');
const InvalidSignaturePublicKeySecurityLevelError = require('../errors/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeySecurityLevelNotMetError = require('../errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('../errors/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledError = require('../errors/PublicKeyIsDisabledError');
const InvalidIdentityPublicKeyTypeError = require('../errors/InvalidIdentityPublicKeyTypeError');

/**
 * Validate state transition signature
 *
 * @param {validateIdentityExistence} validateIdentityExistence
 * @returns {validateStateTransitionIdentitySignature}
 */
function validateStateTransitionIdentitySignatureFactory(
  validateIdentityExistence,
) {
  /**
   * @typedef validateStateTransitionIdentitySignature
   * @param {
   * DataContractCreateTransition|
   * DocumentsBatchTransition
   * } stateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateStateTransitionIdentitySignature(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    // Owner must exist
    const result = await validateIdentityExistence(
      stateTransition.getOwnerId(),
      executionContext,
    );

    if (!result.isValid()) {
      return result;
    }

    // Signature must be valid
    const identity = result.getData();

    const publicKey = identity.getPublicKeyById(stateTransition.getSignaturePublicKeyId());

    if (!publicKey) {
      result.addError(
        new MissingPublicKeyConsensusError(stateTransition.getSignaturePublicKeyId()),
      );

      return result;
    }

    if (
      publicKey.getType() !== IdentityPublicKey.TYPES.ECDSA_SECP256K1
      && publicKey.getType() !== IdentityPublicKey.TYPES.ECDSA_HASH160
    ) {
      result.addError(
        new InvalidIdentityPublicKeyTypeConsensusError(publicKey.getType()),
      );

      return result;
    }

    try {
      const signatureIsValid = await stateTransition.verifySignature(publicKey);

      if (!signatureIsValid) {
        result.addError(
          new InvalidStateTransitionSignatureConsensusError(stateTransition),
        );
      }
    } catch (e) {
      if (e instanceof StateTransitionIsNotSignedError) {
        result.addError(
          new StateTransitionIsNotSignedConsensusError(stateTransition),
        );
      } else if (e instanceof PublicKeyMismatchError) {
        result.addError(
          new PublicKeyMismatchConsensusError(e.getPublicKey()),
        );
      } else if (e instanceof InvalidIdentityPublicKeyTypeError) {
        result.addError(
          new InvalidIdentityPublicKeyTypeConsensusError(e.getPublicKeyType()),
        );
      } else if (e instanceof InvalidSignaturePublicKeySecurityLevelError) {
        result.addError(
          new InvalidSignaturePublicKeySecurityLevelConsensusError(e.getSecurityLevel()),
        );
      } else if (e instanceof PublicKeySecurityLevelNotMetError) {
        result.addError(
          new PublicKeySecurityLevelNotMetConsensusError(
            e.getPublicKeySecurityLevel(),
            e.getKeySecurityLevelRequirement(),
          ),
        );
      } else if (e instanceof WrongPublicKeyPurposeError) {
        result.addError(
          new WrongPublicKeyPurposeConsensusError(
            e.getPublicKeyPurpose(),
            e.getKeyPurposeRequirement(),
          ),
        );
      } else if (e instanceof PublicKeyIsDisabledError) {
        result.addError(
          new PublicKeyIsDisabledConsensusError(e.getPublicKey()),
        );
      } else if (e instanceof DPPError) {
        result.addError(
          new InvalidStateTransitionSignatureConsensusError(stateTransition),
        );
      } else {
        throw e;
      }
    }

    return result;
  }

  return validateStateTransitionIdentitySignature;
}

module.exports = validateStateTransitionIdentitySignatureFactory;
