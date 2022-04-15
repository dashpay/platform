const ValidationResult = require('../../../../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../../../errors/consensus/state/dataContract/DataContractAlreadyPresentError');
const Identity = require('../../../../../identity/Identity');
const IdentityPublicKey = require('../../../../../identity/IdentityPublicKey');
const InvalidSignaturePublicKeyIdError = require('../../../../../errors/consensus/state/identity/InvalidSignaturePublicKeyIdError');

/**
 *
 * @param {StateRepository} stateRepository
 * @return {validateDataContractCreateTransitionState}
 */
function validateDataContractCreateTransitionStateFactory(stateRepository) {
  /**
   * @typedef validateDataContractCreateTransitionState
   * @param {DataContractCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data contract shouldn't exist
    const existingDataContract = await stateRepository.fetchDataContract(dataContractId);

    if (existingDataContract) {
      result.addError(
        new DataContractAlreadyPresentError(dataContractId.toBuffer()),
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

  return validateDataContractCreateTransitionState;
}

module.exports = validateDataContractCreateTransitionStateFactory;
