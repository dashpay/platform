import getWasm from './helpers/wasm.js';
import {
  document, documentTypeName, revision, dataContractId, ownerId, id,
} from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('BatchTransition', () => {
  describe('serialization / deserialization', () => {
    describe('documents', () => {
      it('should allow to create from v0 transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batch = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batch.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create from v1 transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchedTransition = new wasm.BatchedTransitionWASM(documentTransition);

        const batch = wasm.BatchTransitionWASM.fromV1BatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchedTransition.__wbg_ptr).to.not.equal(0);
        expect(batch.__wbg_ptr).to.not.equal(0);
      });
    });
    describe('tokens', () => {
      it('should allow to create from v1 transition', () => {
        const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId);

        const mintTransition = new wasm.TokenMintTransitionWASM(baseTransition, ownerId, BigInt(9999), 'bbbbbb');

        const transition = new wasm.TokenTransitionWASM(mintTransition);

        const batchedTransition = new wasm.BatchedTransitionWASM(transition);

        const batch = wasm.BatchTransitionWASM.fromV1BatchedTransitions([batchedTransition, batchedTransition], ownerId, 1);

        expect(baseTransition.__wbg_ptr).to.not.equal(0);
        expect(mintTransition.__wbg_ptr).to.not.equal(0);
        expect(transition.__wbg_ptr).to.not.equal(0);
        expect(batchedTransition.__wbg_ptr).to.not.equal(0);
        expect(batch.__wbg_ptr).to.not.equal(0);
      });
    });
  });

  describe('getters', () => {
    it('should allow to get transitions', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.transitions.length).to.equal(2);
    });

    it('should allow to get signature', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.signature).to.deep.equal(new Uint8Array(0));
    });

    it('should allow to get signature public key id', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.signaturePublicKeyId).to.equal(1);
    });

    it('should allow to get all purchases amount', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));
      const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

      const documentTransition = createTransition.toDocumentTransition();
      const documentTransition2 = purchaseTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition2], documentInstance.ownerId, 1, 1);

      expect(batchTransition.allPurchasesAmount).to.deep.equal(BigInt(100));
    });

    it('should allow to get owner id', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.ownerId.base58()).to.deep.equal(documentInstance.ownerId.base58());
    });

    it('should allow to get modified data ids', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.modifiedDataIds.map((identifier) => identifier.base58())).to.deep.equal([documentTransition.id.base58(), documentTransition.id.base58()]);
    });

    it('should allow to get allConflictingIndexCollateralVotingFunds', () => {
      const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
      const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

      expect(batchTransition.allConflictingIndexCollateralVotingFunds).to.deep.equal(undefined);
    });
  });
});
