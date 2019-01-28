const SVContract = require('../../stateView/contract/SVContract');

const getDPContractFixture = require('./getDPContractFixture');
const getReferenceFixture = require('./getReferenceFixture');
const getDPObjectsFixture = require('./getDPObjectsFixture');

function getSVContractFixture() {
  const { userId } = getDPObjectsFixture;

  const dpContract = getDPContractFixture();
  const reference = getReferenceFixture();

  const contractId = dpContract.getId();

  return new SVContract(
    contractId,
    userId,
    dpContract,
    reference,
  );
}

module.exports = getSVContractFixture;
