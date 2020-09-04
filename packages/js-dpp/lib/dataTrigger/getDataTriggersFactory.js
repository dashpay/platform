const AbstractDocumentTransition = require('../document/stateTransition/documentTransition/AbstractDocumentTransition');

const DataTrigger = require('./DataTrigger');

const createDomainDataTrigger = require('./dpnsTriggers/createDomainDataTrigger');
const rejectDataTrigger = require('./dpnsTriggers/rejectDataTrigger');

/**
 * Get respective data triggers (factory)
 *
 * @return {getDataTriggers}
 */
function getDataTriggersFactory() {
  const dataTriggers = [
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'domain',
      AbstractDocumentTransition.ACTIONS.CREATE,
      createDomainDataTrigger,
      process.env.DPNS_TOP_LEVEL_IDENTITY,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'domain',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
      process.env.DPNS_TOP_LEVEL_IDENTITY,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'domain',
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
      process.env.DPNS_TOP_LEVEL_IDENTITY,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'preorder',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
      process.env.DPNS_TOP_LEVEL_IDENTITY,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'preorder',
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
      process.env.DPNS_TOP_LEVEL_IDENTITY,
    ),
  ];

  /**
   * Get respective data triggers
   *
   * @typedef getDataTriggers
   *
   * @param {string} dataContractId
   * @param {string} documentType
   * @param {number} transitionAction
   *
   * @returns {DataTrigger[]}
   */
  function getDataTriggers(dataContractId, documentType, transitionAction) {
    return dataTriggers.filter(
      (dataTrigger) => dataTrigger.isMatchingTriggerForData(
        dataContractId,
        documentType,
        transitionAction,
      ),
    );
  }

  return getDataTriggers;
}

module.exports = getDataTriggersFactory;
