const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const hash = require('../../util/hash');
const entropy = require('../../util/entropy');

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
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: ownerId.toBuffer(),
    },
    ...options,
  };

  return factory.create(dataContract, ownerId.toBuffer(), 'preorder', data);
}

module.exports = getPreorderDocumentFixture;
