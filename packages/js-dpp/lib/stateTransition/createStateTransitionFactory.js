const types = require('./stateTransitionTypes');

const DataContractStateTransition = require('../dataContract/stateTransition/DataContractStateTransition');
const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');

/**
 * @param {createDataContract} createDataContract
 * @return {createStateTransition}
 */
function createStateTransitionFactory(createDataContract) {
  /**
   * @typedef createStateTransition
   * @param {RawDataContractStateTransition} rawStateTransition
   * @return {DataContractStateTransition}
   */
  function createStateTransition(rawStateTransition) {
    switch (rawStateTransition.type) {
      case types.DATA_CONTRACT: {
        const dataContract = createDataContract(rawStateTransition.dataContract);

        return new DataContractStateTransition(dataContract);
      }
      default:
        throw new InvalidStateTransitionTypeError(rawStateTransition);
    }
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
