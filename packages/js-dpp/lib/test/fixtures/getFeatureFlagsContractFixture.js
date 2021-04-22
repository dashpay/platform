const featureFlagDocuments = require('@dashevo/feature-flags-contract/schema/feature-flags-documents.json');
const DataContractFactory = require('../../dataContract/DataContractFactory');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();

/**
 * @return {DataContract}
 */
module.exports = function getFeatureFlagsContractFixture() {
  const factory = new DataContractFactory(() => {});
  return factory.create(ownerId, featureFlagDocuments);
};
