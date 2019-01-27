const SVContract = require('../../stateView/contract/SVContract');

const getDPContractFixture = require('./getDPContractFixture');
const getReferenceFixture = require('./getReferenceFixture');

function getSVContractFixture() {
  const dpContract = getDPContractFixture();
  const reference = getReferenceFixture();

  const contractId = dpContract.getId();

  return new SVContract(
    contractId,
    dpContract,
    reference,
  );
}

module.exports = getSVContractFixture;
