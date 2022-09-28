const types = require('./stateTransitionTypes');

const DocumentsBatchTransition = require('../document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');
const DataContractCreateTransition = require('../dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');
const IdentityCreateTransition = require('../identity/stateTransition/IdentityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../identity/stateTransition/IdentityTopUpTransition/IdentityTopUpTransition');
const IdentityUpdateTransition = require('../identity/stateTransition/IdentityUpdateTransition/IdentityUpdateTransition');

const InvalidStateTransitionTypeError = require('./errors/InvalidStateTransitionTypeError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');
const MissingDataContractIdError = require('./errors/MissingDataContractIdError');

const Identifier = require('../identifier/Identifier');
const DataContractUpdateTransition = require('../dataContract/stateTransition/DataContractUpdateTransition/DataContractUpdateTransition');
const StateTransitionExecutionContext = require('./StateTransitionExecutionContext');

const typesToClasses = {
  [types.DATA_CONTRACT_CREATE]: DataContractCreateTransition,
  [types.DATA_CONTRACT_UPDATE]: DataContractUpdateTransition,
  [types.DOCUMENTS_BATCH]: DocumentsBatchTransition,
  [types.IDENTITY_CREATE]: IdentityCreateTransition,
  [types.IDENTITY_TOP_UP]: IdentityTopUpTransition,
  [types.IDENTITY_UPDATE]: IdentityUpdateTransition,
};

/**
 * @param {StateRepository} stateRepository
 * @return {createStateTransition}
 */
function createStateTransitionFactory(stateRepository) {
  /**
   * @typedef {createStateTransition}
   * @param {RawStateTransition} rawStateTransition
   * @param {StateTransitionExecutionContext} [executionContext]
   * @return {Promise<AbstractStateTransition>}
   */
  async function createStateTransition(rawStateTransition, executionContext) {
    if (!typesToClasses[rawStateTransition.type]) {
      throw new InvalidStateTransitionTypeError(rawStateTransition.type);
    }

    if (!executionContext) {
      // eslint-disable-next-line no-param-reassign
      executionContext = new StateTransitionExecutionContext();
    }

    if (rawStateTransition.type === types.DOCUMENTS_BATCH) {
      const dataContractPromises = rawStateTransition.transitions
        .map(async (documentTransition) => {
          if (!Object.prototype.hasOwnProperty.call(documentTransition, '$dataContractId')) {
            throw new MissingDataContractIdError(documentTransition);
          }

          const dataContractId = new Identifier(documentTransition.$dataContractId);

          const dataContract = await stateRepository.fetchDataContract(
            dataContractId,
            executionContext,
          );

          if (!dataContract) {
            throw new DataContractNotPresentError(dataContractId);
          }

          return dataContract;
        });

      const dataContracts = await Promise.all(dataContractPromises);

      const stateTransition = new typesToClasses[rawStateTransition.type](
        rawStateTransition,
        dataContracts,
      );

      stateTransition.setExecutionContext(executionContext);

      return stateTransition;
    }

    const stateTransition = new typesToClasses[rawStateTransition.type](rawStateTransition);

    stateTransition.setExecutionContext(executionContext);

    return stateTransition;
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
