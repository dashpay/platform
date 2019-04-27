const STPacket = require('@dashevo/dpp/lib/stPacket/STPacket');

const getContractFixture = require('./getContractFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

/**
 * @return {STPacket[]}
 */
function getSTPacketsFixture() {
  const contract = getContractFixture();
  const documents = getDocumentsFixture().map(document => document.removeMetadata());

  const contractId = contract.getId();

  return [
    new STPacket(contractId, contract),
    new STPacket(contractId, documents.slice(0, 2)),
    new STPacket(contractId, documents.slice(2)),
  ];
}

module.exports = getSTPacketsFixture;
