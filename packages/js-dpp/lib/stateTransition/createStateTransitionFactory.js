const types = require('./stateTransitionTypes');

const DocumentsBatchTransition = require('../document/stateTransition/DocumentsBatchTransition');
const DataContractCreateTransition = require('../dataContract/stateTransition/DataContractCreateTransition');
const IdentityCreateTransition = require('../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');
const InvalidDataContractIdError = require('../errors/InvalidDataContractIdError');

const typesToClasses = {
  [types.DATA_CONTRACT_CREATE]: DataContractCreateTransition,
  [types.DOCUMENTS_BATCH]: DocumentsBatchTransition,
  [types.IDENTITY_CREATE]: IdentityCreateTransition,
  [types.IDENTITY_TOP_UP]: IdentityTopUpTransition,
};

/**
 * @param {StateRepository} stateRepository
 *
 * @return {createStateTransition}
 */
function createStateTransitionFactory(stateRepository) {
  /**
   * @typedef createStateTransition
   *
   * @param {
   *   RawDataContractCreateTransition |
   *   RawDocumentsBatchTransition |
   *   RawIdentityCreateTransition |
   *   RawIdentityTopUpTransition
   * } rawStateTransition
   * @param {Object} [options]
   * @param {boolean} [options.fromJSON]
   *
   * @return {
   *   Promise<DataContractCreateTransition | DocumentsBatchTransition | IdentityCreateTransition>
   * }
   */
  async function createStateTransition(rawStateTransition, options = {}) {
    const fromJSON = !!options.fromJSON;

    if (!typesToClasses[rawStateTransition.type]) {
      throw new InvalidStateTransitionTypeError(rawStateTransition);
    }

    if (rawStateTransition.type === types.DOCUMENTS_BATCH) {
      const dataContractPromises = rawStateTransition.transitions
        .map(async (documentTransition) => {
          if (!documentTransition.$dataContractId) {
            throw new InvalidDataContractIdError(documentTransition.$dataContractId);
          }

          const dataContract = await stateRepository.fetchDataContract(
            documentTransition.$dataContractId,
          );

          if (!dataContract) {
            throw new DataContractNotPresentError(rawStateTransition.$dataContractId);
          }

          return dataContract;
        });

      const dataContracts = await Promise.all(dataContractPromises);

      if (fromJSON) {
        return typesToClasses[rawStateTransition.type].fromJSON(rawStateTransition, dataContracts);
      }

      return new typesToClasses[rawStateTransition.type](rawStateTransition, dataContracts);
    }

    if (fromJSON) {
      return typesToClasses[rawStateTransition.type].fromJSON(rawStateTransition);
    }

    return new typesToClasses[rawStateTransition.type](rawStateTransition);
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
