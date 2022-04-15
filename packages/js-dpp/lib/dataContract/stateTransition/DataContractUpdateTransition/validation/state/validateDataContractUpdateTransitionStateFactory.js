const ValidationResult = require('../../../../../validation/ValidationResult');

const InvalidDataContractVersionError = require('../../../../../errors/consensus/basic/dataContract/InvalidDataContractVersionError');
const DataContractNotPresentError = require('../../../../../errors/consensus/basic/document/DataContractNotPresentError');
const Identity = require('../../../../../identity/Identity');
const IdentityPublicKey = require('../../../../../identity/IdentityPublicKey');
const InvalidSignaturePublicKeyIdError = require('../../../../../errors/consensus/state/identity/InvalidSignaturePublicKeyIdError');

/**
 *
 * @param {StateRepository} stateRepository
 * @return {validateDataContractUpdateTransitionState}
 */
function validateDataContractUpdateTransitionStateFactory(
  stateRepository,
) {
  /**
   * @typedef validateDataContractUpdateTransitionState
   * @param {DataContractCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractUpdateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data contract should exist
    const existingDataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!existingDataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId.toBuffer()),
      );

      return result;
    }

    // Version difference should be exactly 1
    const oldVersion = existingDataContract.getVersion();
    const newVersion = dataContract.getVersion();
    const versionDiff = newVersion - oldVersion;

    if (versionDiff !== 1) {
      result.addError(
        new InvalidDataContractVersionError(
          oldVersion + 1,
          oldVersion + versionDiff,
        ),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const identityId = stateTransition.getIdentityId();
    const storedIdentity = await stateRepository.fetchIdentity(identityId);

    // copy identity
    const identity = new Identity(storedIdentity.toObject());

    if (stateTransition.getBIP16Script()) {
      const publicKey = identity.getPublicKeyById(stateTransition.getSignaturePublicKeyId());
      if (publicKey.getType() !== IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH) {
        result.addError(
          new InvalidSignaturePublicKeyIdError(stateTransition.getSignaturePublicKeyId()),
        );

        return result;
      }
    }

    return result;
  }

  return validateDataContractUpdateTransitionState;
}

module.exports = validateDataContractUpdateTransitionStateFactory;
