const types = require('./stateTransitionTypes');

const DocumentsBatchTransition = require('../document/stateTransition/DocumentsBatchTransition');
const DataContractCreateTransition = require('../dataContract/stateTransition/DataContractCreateTransition');
const IdentityCreateTransition = require('../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');
const MissingDataContractIdError = require('../errors/MissingDataContractIdError');

const Identifier = require('../identifier/Identifier');

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
   * @param {RawStateTransition} rawStateTransition
   *
   * @return {Promise<AbstractStateTransition>}
   */
  async function createStateTransition(rawStateTransition) {
    if (!typesToClasses[rawStateTransition.type]) {
      throw new InvalidStateTransitionTypeError(rawStateTransition);
    }

    if (rawStateTransition.type === types.DOCUMENTS_BATCH) {
      const dataContractPromises = rawStateTransition.transitions
        .map(async (documentTransition) => {
          if (!Object.prototype.hasOwnProperty.call(documentTransition, '$dataContractId')) {
            throw new MissingDataContractIdError(documentTransition);
          }

          const dataContractId = new Identifier(documentTransition.$dataContractId);

          const dataContract = await stateRepository.fetchDataContract(dataContractId);

          if (!dataContract) {
            throw new DataContractNotPresentError(rawStateTransition.$dataContractId);
          }

          return dataContract;
        });

      const dataContracts = await Promise.all(dataContractPromises);

      return new typesToClasses[rawStateTransition.type](rawStateTransition, dataContracts);
    }

    return new typesToClasses[rawStateTransition.type](rawStateTransition);
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
