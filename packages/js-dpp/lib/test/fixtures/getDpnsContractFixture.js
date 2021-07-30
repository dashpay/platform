const dpnsDocuments = require('@dashevo/dpns-contract/schema/dpns-contract-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');
const createDPPMock = require('../mocks/createDPPMock');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  const factory = new DataContractFactory(createDPPMock(), () => {});
  return factory.create(ownerId, dpnsDocuments);
};
