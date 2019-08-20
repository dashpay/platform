const Document = require('../document/Document');

const DataTrigger = require('./DataTrigger');

const createDomainDataTrigger = require('./dpnsTriggers/createDomainDataTrigger');
const deleteDomainDataTrigger = require('./dpnsTriggers/deleteDomainDataTrigger');
const updateDomainDataTrigger = require('./dpnsTriggers/updateDomainDataTrigger');

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
      Document.ACTIONS.CREATE,
      createDomainDataTrigger,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'domain',
      Document.ACTIONS.UPDATE,
      updateDomainDataTrigger,
    ),
    new DataTrigger(
      process.env.DPNS_CONTRACT_ID,
      'domain',
      Document.ACTIONS.DELETE,
      deleteDomainDataTrigger,
    ),
  ];

  /**
   * Get respective data triggers
   *
   * @typedef getDataTriggers
   *
   * @param {sting} contractId
   * @param {number} documentType
   * @param {number} documentAction
   *
   * @returns {DataTrigger[]}
   */
  function getDataTriggers(contractId, documentType, documentAction) {
    return dataTriggers.filter(
      dataTrigger => dataTrigger.isMatchingTriggerForData(contractId, documentType, documentAction),
    );
  }

  return getDataTriggers;
}

module.exports = getDataTriggersFactory;
