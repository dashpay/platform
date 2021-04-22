const getFeatureFlagsContractFixture = require('./getFeatureFlagsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const ownerId = generateRandomIdentifier();
const dataContract = getFeatureFlagsContractFixture();

/**
 * @return {Document}
 */
function getFeatureFlagsDocumentsFixture() {
  const factory = new DocumentFactory(
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  return [
    factory.create(dataContract, ownerId, 'fixCumulativeFeesBug', {
      enabled: true,
      enableAtHeight: 77,
    }),
  ];
}

module.exports = getFeatureFlagsDocumentsFixture;

module.exports.dataContract = dataContract;
