const dpnsDocuments = require('@dashevo/dpns-contract/src/schema/dpns-documents');
const DataContractFactory = require('../../dataContract/DataContractFactory');

const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  const factory = new DataContractFactory(() => {});
  return factory.create(ownerId, dpnsDocuments);
};
