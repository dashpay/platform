const SVContract = require('../../../../lib/stateView/contract/SVContract');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('SVContract', () => {
  let svContract;
  let dpContract;
  let contractId;
  let reference;
  let isDeleted;
  let previousRevisions;

  beforeEach(() => {
    dpContract = getDPContractFixture();
    reference = getReferenceFixture();

    contractId = dpContract.getId();
    isDeleted = false;
    previousRevisions = [];

    svContract = new SVContract(
      contractId,
      dpContract,
      reference,
      isDeleted,
      previousRevisions,
    );
  });

  describe('#getContractId', () => {
    it('should return contract ID', () => {
      const result = svContract.getContractId();

      expect(result).to.be.equal(contractId);
    });
  });

  describe('#getDPContract', () => {
    it('should return DP Contract', () => {
      const result = svContract.getDPContract();

      expect(result.toJSON()).to.be.deep.equal(dpContract.toJSON());
    });
  });

  describe('#isDeleted', () => {
    it('should return true if contract is deleted', () => {
      const result = svContract.isDeleted();

      expect(result).to.be.equal(isDeleted);
    });
  });

  describe('#markAsDeleted', () => {
    it('should mark SV Contract as deleted', () => {
      const result = svContract.markAsDeleted();

      expect(result).to.be.equal(svContract);

      expect(svContract.deleted).to.be.true();
    });
  });

  describe('#toJSON', () => {
    it('should return SV Contract as plain object', () => {
      svContract = new SVContract(
        contractId,
        dpContract,
        reference,
        isDeleted,
        previousRevisions,
      );

      expect(svContract.toJSON()).to.be.deep.equal({
        contractId,
        dpContract: dpContract.toJSON(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
      });
    });
  });
});
