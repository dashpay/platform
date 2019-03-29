const SVContract = require('../../stateView/contract/SVContract');

const getContractFixture = require('./getContractFixture');
const getReferenceFixture = require('./getReferenceFixture');
const getDocumentsFixture = require('./getDocumentsFixture');

function getSVContractFixture() {
  const { userId } = getDocumentsFixture;

  const contract = getContractFixture();
  const reference = getReferenceFixture();

  const contractId = contract.getId();

  return new SVContract(
    contractId,
    userId,
    contract,
    reference,
  );
}

module.exports = getSVContractFixture;
