const crypto = require('crypto');
const getDpnsContractFixture = require('./getDpnsContractFixture');
const DocumentFactory = require('../../document/DocumentFactory');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');
const createDPPMock = require('../mocks/createDPPMock');

const ownerId = generateRandomIdentifier();
const dataContract = getDpnsContractFixture();

/**
 * @return {Document}
 */
function getTopDocumentFixture(options = {}) {
  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  const label = options.label || 'grandparent';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const data = {
    label,
    normalizedLabel,
    normalizedParentDomainName: '',
    preorderSalt: crypto.randomBytes(32),
    records: {
      dashUniqueIdentityId: ownerId,
    },
    subdomainRules: {
      allowSubdomains: true,
    },
    ...options,
  };

  return factory.create(dataContract, ownerId, 'domain', data);
}

/**
 * @return {Document}
 */
function getParentDocumentFixture(options = {}) {
  const factory = new DocumentFactory(
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  const label = options.label || 'Parent';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const data = {
    label,
    normalizedLabel,
    normalizedParentDomainName: 'grandparent',
    preorderSalt: crypto.randomBytes(32),
    records: {
      dashUniqueIdentityId: ownerId,
    },
    subdomainRules: {
      allowSubdomains: false,
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
    createDPPMock(),
    () => ({
      isValid: () => true,
    }),
    () => {},
  );

  const label = options.label || 'Child';
  const normalizedLabel = options.normalizedLabel || label.toLowerCase();
  const parent = getParentDocumentFixture();
  const parentDomainName = `${parent.getData().normalizedLabel}.${parent.getData().normalizedParentDomainName}`;
  const data = {
    label,
    normalizedLabel,
    normalizedParentDomainName: parentDomainName,
    preorderSalt: crypto.randomBytes(32),
    records: {
      dashUniqueIdentityId: ownerId,
    },
    subdomainRules: {
      allowSubdomains: false,
    },
    ...options,
  };

  return factory.create(dataContract, ownerId, 'domain', data);
}

module.exports = {
  getTopDocumentFixture,
  getParentDocumentFixture,
  getChildDocumentFixture,
};

module.exports.dataContract = dataContract;
