const getFeatureFlagsContractFixture = require('./getFeatureFlagsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const createDPPMock = require('../mocks/createDPPMock');

const ownerId = generateRandomIdentifier();

/**
 * @return {Document}
 */
function getFeatureFlagsDocumentsFixture(dataContract = getFeatureFlagsContractFixture()) {
  const factory = new DocumentFactory(
    createDPPMock(),
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
