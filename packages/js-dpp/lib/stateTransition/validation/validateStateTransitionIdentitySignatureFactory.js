const IdentityPublicKey = require('../../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeConsensusError = require('../../errors/consensus/signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureConsensusError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyConsensusError = require('../../errors/consensus/signature/MissingPublicKeyError');
const InvalidSignaturePublicKeySecurityLevelConsensusError = require('../../errors/consensus/signature/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeySecurityLevelNotMetConsensusError = require('../../errors/consensus/signature/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeConsensusError = require('../../errors/consensus/signature/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledConsensusError = require('../../errors/consensus/signature/PublicKeyIsDisabledError');
const DPPError = require('../../errors/DPPError');
const InvalidSignaturePublicKeySecurityLevelError = require('../errors/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeySecurityLevelNotMetError = require('../errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('../errors/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledError = require('../errors/PublicKeyIsDisabledError');
const SignatureVerificationOperation = require('../fee/operations/SignatureVerificationOperation');
const ValidationResult = require('../../validation/ValidationResult');
const IdentityNotFoundError = require('../../errors/consensus/signature/IdentityNotFoundError');
const StateTransitionExecutionContext = require('../StateTransitionExecutionContext');
const InvalidIdentityPublicKeyTypeError = require('../errors/InvalidIdentityPublicKeyTypeError');

const supportedPublicKeyTypes = [
  IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  IdentityPublicKey.TYPES.BLS12_381,
  IdentityPublicKey.TYPES.ECDSA_HASH160,
];

/**
 * Validate state transition signature
 *
 * @param {StateRepository} stateRepository
 * @returns {validateStateTransitionIdentitySignature}
 */
function validateStateTransitionIdentitySignatureFactory(
  stateRepository,
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
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();

    const ownerId = stateTransition.getOwnerId();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // identity to proceed with following logic
    const tmpExecutionContext = new StateTransitionExecutionContext();

    // Owner must exist
    const identity = await stateRepository.fetchIdentity(ownerId, tmpExecutionContext);

    // Collect operations back from temporary context
    executionContext.addOperation(...tmpExecutionContext.getOperations());

    if (!identity) {
      result.addError(new IdentityNotFoundError(ownerId.toBuffer()));

      return result;
    }

    // Signature must be valid
    const publicKey = identity.getPublicKeyById(stateTransition.getSignaturePublicKeyId());

    if (!publicKey) {
      result.addError(
        new MissingPublicKeyConsensusError(stateTransition.getSignaturePublicKeyId()),
      );

      return result;
    }

    if (!supportedPublicKeyTypes.includes(publicKey.getType())) {
      result.addError(
        new InvalidIdentityPublicKeyTypeConsensusError(publicKey.getType()),
      );

      return result;
    }

    const operation = new SignatureVerificationOperation(publicKey.getType());

    executionContext.addOperation(operation);

    if (executionContext.isDryRun()) {
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
      if (e instanceof InvalidSignaturePublicKeySecurityLevelError) {
        result.addError(
          new InvalidSignaturePublicKeySecurityLevelConsensusError(
            e.getPublicKeySecurityLevel(),
            e.getKeySecurityLevelRequirement(),
          ),
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
          new PublicKeyIsDisabledConsensusError(e.getPublicKey().getId()),
        );
      } else if (e instanceof InvalidIdentityPublicKeyTypeError) {
        result.addError(
          new InvalidIdentityPublicKeyTypeConsensusError(e.getPublicKeyType()),
        );
      } else if (e instanceof DPPError) {
        result.addError(
          new InvalidStateTransitionSignatureConsensusError(),
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
