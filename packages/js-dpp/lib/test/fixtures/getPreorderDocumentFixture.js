const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const hash = require('../../../lib/util/hash');
const entropy = require('../../../lib/util/entropy');

const generateRandomId = require('../utils/generateRandomId');

const userId = generateRandomId();

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
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: userId,
    },
    ...options,
  };

  return factory.create(dataContract, userId, 'preorder', data);
}

module.exports = getPreorderDocumentFixture;
