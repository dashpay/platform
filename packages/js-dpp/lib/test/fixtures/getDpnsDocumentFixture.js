const { Transaction, PrivateKey } = require('@dashevo/dashcore-lib');
const entropy = require('../../../lib/util/entropy');
const multihash = require('../../../lib/util/multihashDoubleSHA256');
const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');

const transaction = new Transaction().setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER);
transaction.extraPayload.setUserName('MyUser').setPubKeyIdFromPrivateKey(new PrivateKey());

const userId = transaction.hash;

/**
 * @return {Document}
 */
function getParentDocumentFixture(options = {}) {
  const dataContract = getDpnsContractFixture();

  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  const label = options.label || 'Parent';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const fullDomainName = `${normalizedLabel}.grandparent`;
  const data = Object.assign({}, {
    nameHash: multihash.hash(Buffer.from(fullDomainName)).toString('hex'),
    label,
    normalizedLabel,
    normalizedParentDomainName: 'grandparent',
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: transaction.hash,
    },
  }, options);

  return factory.create(dataContract, userId, 'domain', data);
}

/**
 * @return {Document}
 */
function getChildDocumentFixture(options = {}) {
  const dataContract = getDpnsContractFixture();

  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  const label = options.label || 'Child';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const parent = getParentDocumentFixture();
  const parentDomainName = `${parent.getData().normalizedLabel}.${parent.getData().normalizedParentDomainName}`;
  const fullDomainName = `${normalizedLabel}.${parentDomainName}`;
  const data = Object.assign({}, {
    nameHash: multihash.hash(Buffer.from(fullDomainName)).toString('hex'),
    label,
    normalizedLabel,
    normalizedParentDomainName: parentDomainName,
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: transaction.hash,
    },
  }, options);

  return factory.create(dataContract, userId, 'domain', data);
}

module.exports = {
  getParentDocumentFixture,
  getChildDocumentFixture,
};
