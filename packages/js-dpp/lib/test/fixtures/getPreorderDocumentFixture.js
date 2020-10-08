const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const hash = require('../../util/hash');
const generateEntropy = require('../../util/generateEntropy');

const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();

/**
 * @return {Document}
 */
function getPreorderDocumentFixture(options = {}) {
  const dataContract = getDpnsContractFixture();

  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  const label = options.label || 'Preorder';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const data = {
    hash: hash(Buffer.from(normalizedLabel)).toString('hex'),
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
