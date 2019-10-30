const SVContract = require('../../../../lib/stateView/contract/SVContract');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('SVContract', () => {
  let svContract;
  let contract;
  let contractId;
  let reference;
  let isDeleted;
  let previousRevisions;

  beforeEach(() => {
    contract = getDataContractFixture();
    reference = getReferenceFixture();

    contractId = contract.getId();
    isDeleted = false;
    previousRevisions = [];

    svContract = new SVContract(
      contract,
      reference,
      isDeleted,
      previousRevisions,
    );
  });

  describe('#getId', () => {
    it('should return contract ID', () => {
      const result = svContract.getId();

      expect(result).to.equal(contractId);
    });
  });

  describe('#getDataContract', () => {
    it('should return Data Contract', () => {
      const result = svContract.getDataContract();

      expect(result.toJSON()).to.deep.equal(contract.toJSON());
    });
  });

  describe('#isDeleted', () => {
    it('should return true if contract is deleted', () => {
      const result = svContract.isDeleted();

      expect(result).to.equal(isDeleted);
    });
  });

  describe('#markAsDeleted', () => {
    it('should mark SV Contract as deleted', () => {
      const result = svContract.markAsDeleted();

      expect(result).to.equal(svContract);

      expect(svContract.deleted).to.be.true();
    });
  });

  describe('#toJSON', () => {
    it('should return SV Contract as a plain object', () => {
      svContract = new SVContract(
        contract,
        reference,
        isDeleted,
        previousRevisions,
      );

      expect(svContract.toJSON()).to.deep.equal({
        contractId,
        contract: contract.toJSON(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
      });
    });
  });
});
