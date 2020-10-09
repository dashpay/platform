const dpnsDocuments = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  const factory = new DataContractFactory(() => {});
  return factory.create(ownerId, dpnsDocuments);
};
