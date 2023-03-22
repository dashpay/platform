const featureFlagDocuments = require('@dashevo/feature-flags-contract/schema/feature-flags-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');
const createDPPMock = require('../mocks/createDPPMock');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getFeatureFlagsContractFixture() {
  const factory = new DataContractFactory(createDPPMock(), () => {});
  return factory.create(ownerId, featureFlagDocuments);
};
