const entropy = require('../../util/entropy');
const multihash = require('../../util/multihashDoubleSHA256');
const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateRandomId = require('../utils/generateRandomId');

const ownerId = generateRandomId();
const dataContract = getDpnsContractFixture();

/**
 * @return {Document}
 */
function getParentDocumentFixture(options = {}) {
  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  const label = options.label || 'Parent';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const fullDomainName = `${normalizedLabel}.grandparent`;
  const data = {
    nameHash: multihash.hash(Buffer.from(fullDomainName)).toString('hex'),
    label,
    normalizedLabel,
    normalizedParentDomainName: 'grandparent',
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: ownerId,
    },
    ...options,
  };

  return factory.create(dataContract, ownerId, 'domain', data);
}

/**
 * @return {Document}
 */
function getChildDocumentFixture(options = {}) {
  const factory = new DocumentFactory(
    () => {},
    () => {},
  );

  const label = options.label || 'Child';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const parent = getParentDocumentFixture();
  const parentDomainName = `${parent.getData().normalizedLabel}.${parent.getData().normalizedParentDomainName}`;
  const fullDomainName = `${normalizedLabel}.${parentDomainName}`;
  const data = {
    nameHash: multihash.hash(Buffer.from(fullDomainName)).toString('hex'),
    label,
    normalizedLabel,
    normalizedParentDomainName: parentDomainName,
    preorderSalt: entropy.generate(),
    records: {
      dashIdentity: ownerId,
    },
    ...options,
  };

  return factory.create(dataContract, ownerId, 'domain', data);
}

module.exports = {
  getParentDocumentFixture,
  getChildDocumentFixture,
};

module.exports.dataContract = dataContract;
