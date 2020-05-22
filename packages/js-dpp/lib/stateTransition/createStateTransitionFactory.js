const types = require('./stateTransitionTypes');

const DocumentsBatchTransition = require('../document/stateTransition/DocumentsBatchTransition');
const DataContractCreateTransition = require('../dataContract/stateTransition/DataContractCreateTransition');
const IdentityCreateTransition = require('../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');

const typesToClasses = {
  [types.DATA_CONTRACT_CREATE]: DataContractCreateTransition,
  [types.DOCUMENTS_BATCH]: DocumentsBatchTransition,
  [types.IDENTITY_CREATE]: IdentityCreateTransition,
  [types.IDENTITY_TOP_UP]: IdentityTopUpTransition,
};

/**
 * @return {createStateTransition}
 */
function createStateTransitionFactory() {
  /**
   * @typedef createStateTransition
   * @param {
   * RawDataContractCreateTransition|
   * RawDocumentsBatchTransition|
   * RawIdentityCreateTransition|
   * RawIdentityTopUpTransition
   * } rawStateTransition
   * @return {DataContractCreateTransition|DocumentsBatchTransition|IdentityCreateTransition}
   */
  function createStateTransition(rawStateTransition) {
    if (!typesToClasses[rawStateTransition.type]) {
      throw new InvalidStateTransitionTypeError(rawStateTransition);
    }

    return new typesToClasses[rawStateTransition.type](rawStateTransition);
  }

  return createStateTransition;
}

module.exports = createStateTransitionFactory;
