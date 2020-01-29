const dpnsDocuments = require('@dashevo/dpns-contract/src/schema/dpns-documents');
const DataContract = require('../../dataContract/DataContract');

const generateRandomId = require('../utils/generateRandomId');

const contractId = generateRandomId();

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  return new DataContract(contractId, dpnsDocuments);
};
