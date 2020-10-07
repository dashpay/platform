const dpnsDocuments = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');

const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  const factory = new DataContractFactory(() => {});
  return factory.create(ownerId.toBuffer(), dpnsDocuments);
};
