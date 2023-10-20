const { Transaction } = require('@dashevo/dashcore-lib');
const { hash } = require('../../../../../lib/utils/hash');
const { InstantAssetLockProof, Identifier } = require('../../../../../dist');
const getInstantAssetLockProofFixture = require('../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');

describe('InstantAssetLockProof', () => {
  let instantAssetLockProof;
  let rawInstantAssetLockProof;

  before(async () => {
    rawInstantAssetLockProof = (await getInstantAssetLockProofFixture())
      .toObject();
    instantAssetLockProof = new InstantAssetLockProof(
      rawInstantAssetLockProof,
    );
  });

  describe('#getOutputIndex', () => {
    it('should return correct type', () => {
      expect(instantAssetLockProof.getOutputIndex())
        .to.equal(rawInstantAssetLockProof.outputIndex);
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      const { transaction: rawTx, outputIndex } = rawInstantAssetLockProof;
      const tx = new Transaction(rawTx);

      const expectedOutPoint = Buffer.from([
        ...Buffer.from(tx.hash, 'hex'),
        ...Buffer.alloc(4, outputIndex),
      ]);

      expect(instantAssetLockProof.getOutPoint())
        .to.deep.equal(expectedOutPoint);
    });
  });

  describe('#getOutput', () => {
    it('should return correct output', () => {
      const { transaction: rawTx, outputIndex } = rawInstantAssetLockProof;
      const tx = new Transaction(rawTx);

      const expectedOutput = {
        ...tx.outputs[outputIndex].toObject(),
      };

      expect(instantAssetLockProof.getOutput())
        .to.deep.equal(expectedOutput);
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = instantAssetLockProof.createIdentifier();

      const expectedIdentifier = Identifier.from(hash(
        instantAssetLockProof.getOutPoint(),
      ));
      expect(identifier)
        .to.deep.equal(expectedIdentifier);
    });
  });

  describe('#getInstantLock', () => {
    it('should return correct instant lock', () => {
      const instantLock = instantAssetLockProof.getInstantLock();

      expect(instantLock)
        .to.deep.equal(rawInstantAssetLockProof.instantLock);
    });
  });

  describe('#getTransaction', () => {
    it('should return correct transaction', () => {
      const transaction = instantAssetLockProof.getTransaction();

      expect(transaction)
        .to.deep.equal(rawInstantAssetLockProof.transaction);
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(instantAssetLockProof.toObject())
        .to.deep.equal({
          instantLock: instantAssetLockProof.getInstantLock(),
          outputIndex: instantAssetLockProof.getOutputIndex(),
          transaction: instantAssetLockProof.getTransaction(),
          type: instantAssetLockProof.getType(),
        });
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(instantAssetLockProof.toJSON())
        .to.deep.equal({
          instantLock: instantAssetLockProof.getInstantLock().toString('base64'),
          outputIndex: instantAssetLockProof.getOutputIndex(),
          transaction: instantAssetLockProof.getTransaction().toString('hex'),
          type: instantAssetLockProof.getType(),
        });
    });
  });
});
