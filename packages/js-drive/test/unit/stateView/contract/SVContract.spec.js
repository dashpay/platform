const SVContract = require('../../../../lib/stateView/contract/SVContract');

const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

describe('SVContract', () => {
  let svContract;
  let contract;
  let contractId;
  let reference;
  let isDeleted;
  let userId;
  let previousRevisions;

  beforeEach(() => {
    ({ userId } = getDocumentsFixture);

    contract = getContractFixture();
    reference = getReferenceFixture();

    contractId = contract.getId();
    isDeleted = false;
    previousRevisions = [];

    svContract = new SVContract(
      contractId,
      userId,
      contract,
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

  describe('#getContract', () => {
    it('should return Contract', () => {
      const result = svContract.getContract();

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
        contractId,
        userId,
        contract,
        reference,
        isDeleted,
        previousRevisions,
      );

      expect(svContract.toJSON()).to.deep.equal({
        contractId,
        userId,
        contract: contract.toJSON(),
        reference: reference.toJSON(),
        isDeleted,
        previousRevisions,
      });
    });
  });
});
