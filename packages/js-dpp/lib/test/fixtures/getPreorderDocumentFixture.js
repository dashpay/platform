const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateEntropy = require('../../util/generateEntropy');

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

  const label = options.label || 'Preorder';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const data = {
    label,
    normalizedLabel,
    parentDomainHash: '',
    preorderSalt: generateEntropy(),
    records: {
      dashIdentity: ownerId,
    },
    ...options,
  };

  return factory.create(dataContract, ownerId, 'preorder', data);
}

module.exports = getPreorderDocumentFixture;
