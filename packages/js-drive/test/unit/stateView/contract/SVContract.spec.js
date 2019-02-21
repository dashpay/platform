const SVContract = require('../../../../lib/stateView/contract/SVContract');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');
const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');

describe('SVContract', () => {
  let svContract;
  let dpContract;
  let contractId;
  let reference;
  let isDeleted;
  let userId;
  let previousRevisions;

  beforeEach(() => {
    ({ userId } = getDPObjectsFixture);

    dpContract = getDPContractFixture();
    reference = getReferenceFixture();

    contractId = dpContract.getId();
    isDeleted = false;
    previousRevisions = [];

    svContract = new SVContract(
      contractId,
      userId,
      dpContract,
      reference,
      isDeleted,
      previousRevisions,
    );
  });

  describe('#getContractId', () => {
    it('should return contract ID', () => {
      const result = svContract.getContractId();

      expect(result).to.equal(contractId);
    });
  });

  describe('#getUserId', () => {
    it('should return user ID', () => {
      const result = svContract.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('#getDPContract', () => {
    it('should return DP Contract', () => {
      const result = svContract.getDPContract();

      expect(result.toJSON()).to.deep.equal(dpContract.toJSON());
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
        contractId,
        userId,
        dpContract,
        reference,
        isDeleted,
        previousRevisions,
      );

      expect(svContract.toJSON()).to.deep.equal({
        contractId,
        userId,
        dpContract: dpContract.toJSON(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
      });
    });
  });
});
