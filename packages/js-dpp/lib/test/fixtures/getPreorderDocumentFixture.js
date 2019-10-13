const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const hash = require('../../../lib/util/hash');
const entropy = require('../../../lib/util/entropy');

const userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';
/**
 * @return {Document}
 */
function getPreorderDocumentFixture(options = {}) {
  const dataContract = getDpnsContractFixture();

  const validateDocumentStub = () => {};

  const factory = new DocumentFactory(
    userId,
    dataContract,
    validateDocumentStub,
  );

  const label = options.label || 'Preorder';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const data = Object.assign({}, {
    hash: hash(Buffer.from(normalizedLabel)).toString('hex'),
    label,
    normalizedLabel,
    parentDomainHash: '',
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: userId,
    },
  }, options);

  return factory.create('preorder', data);
}

module.exports = getPreorderDocumentFixture;
