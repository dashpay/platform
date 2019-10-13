const dpnsDocuments = require('@dashevo/dpns-contract/src/schema/dpns-documents');
const DataContract = require('../../dataContract/DataContract');

/**
 * @return {DataContract}
 */
module.exports = function getDataContractFixture() {
  return new DataContract('b78cd4a40369e401943b7c641ff560ce7069787b', dpnsDocuments);
};
