const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const { generate: generateEntropy } = require('../../util/entropyGenerator');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const createDPPMock = require('../mocks/createDPPMock');

const ownerId = generateRandomIdentifier();

/**
 * @return {Document}
 */
function getPreorderDocumentFixture(options = {}) {
  const dataContract = getDpnsContractFixture();

  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  const data = {
    saltedDomainHash: generateEntropy(),
    ...options,
  };

  return factory.create(dataContract, ownerId, 'preorder', data);
}

module.exports = getPreorderDocumentFixture;
