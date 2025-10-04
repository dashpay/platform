import getWasm from './helpers/wasm.js';
import {
  document, documentTypeName, revision, dataContractId, ownerId, id,
} from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('DocumentsTransitions', () => {
  describe('serialization / deserialization', () => {
    describe('document Create transition', () => {
      it('should allow to create CreateTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from Create transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().createTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(createTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document Delete transition', () => {
      it('should allow to create DeleteTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(deleteTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from Delete transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(deleteTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(deleteTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().deleteTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(deleteTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document Replace transition', () => {
      it('should allow to create ReplaceTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(replaceTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from Replace transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        expect(replaceTransition.__wbg_ptr).to.not.equal(0);
        expect(replaceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(replaceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().replaceTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(replaceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document Transfer transition', () => {
      it('should allow to create ReplaceTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(transferTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from Replace transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        expect(transferTransition.__wbg_ptr).to.not.equal(0);
        expect(transferTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(transferTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().transferTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(transferTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document UpdatePrice transition', () => {
      it('should allow to create UpdatePriceTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(updatePriceTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from UpdatePrice transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(updatePriceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(updatePriceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().updatePriceTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(updatePriceTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document Purchase transition', () => {
      it('should allow to create PurchaseTransition from document', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(purchaseTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Transition from PurchaseTransition transition', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(purchaseTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(purchaseTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransitionWASM.fromV0Transitions([documentTransition, documentTransition], documentInstance.ownerId, 1, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransitionWASM.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().purchaseTransition;

        expect(documentInstance.__wbg_ptr).to.not.equal(0);
        expect(purchaseTransition.__wbg_ptr).to.not.equal(0);
        expect(documentTransition.__wbg_ptr).to.not.equal(0);
        expect(batchTransition.__wbg_ptr).to.not.equal(0);
        expect(st.__wbg_ptr).to.not.equal(0);
        expect(deserializedBatch.__wbg_ptr).to.not.equal(0);
        expect(deserializedPurchaseTransition.__wbg_ptr).to.not.equal(0);
      });
    });
  });
  describe('getters', () => {
    describe('document Create transition', () => {
      it('get data', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        expect(createTransition.data).to.deep.equal(document);
      });

      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        expect(createTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });

      it('get entropy', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        expect(createTransition.entropy).to.deep.equal(documentInstance.entropy);
      });

      it('get prefunded voting balance', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        expect(createTransition.prefundedVotingBalance).to.equal(undefined);
      });
    });

    describe('document Delete transition', () => {
      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        expect(deleteTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });
    });

    describe('document Replace transition', () => {
      it('get data', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        expect(replaceTransition.data).to.deep.equal(document);
      });

      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        expect(replaceTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });

      it('get revision', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        expect(replaceTransition.revision).to.equal(BigInt(2));
      });
    });

    describe('document Transfer transition', () => {
      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(transferTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });

      it('get recipient', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(transferTransition.recipientId.base58()).to.deep.equal(documentInstance.ownerId.base58());
      });
    });

    describe('document Update Price transition', () => {
      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(updatePriceTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });

      it('get price', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(updatePriceTransition.price).to.deep.equal(BigInt(100));
      });
    });

    describe('document Purchase transition', () => {
      it('get base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(purchaseTransition.base.constructor.name).to.equal('DocumentBaseTransitionWASM');
      });

      it('get price', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        expect(purchaseTransition.price).to.deep.equal(BigInt(100));
      });
    });
  });

  describe('setters', () => {
    describe('document Create transition', () => {
      it('set data', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const newData = { message: 'bebra' };

        createTransition.data = newData;

        expect(createTransition.data).to.deep.equal(newData);
      });

      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        createTransition.base = newBase;

        expect(createTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });

      it('set entropy', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const newEntropy = new Uint8Array(32);

        createTransition.entropy = newEntropy;

        expect(createTransition.entropy).to.deep.equal(newEntropy);
      });

      it('set prefunded voting balance', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const createTransition = new wasm.DocumentCreateTransitionWASM(documentInstance, BigInt(1));

        const newPrefundedVotingBalance = new wasm.PrefundedVotingBalanceWASM('note', BigInt(9999));

        createTransition.prefundedVotingBalance = newPrefundedVotingBalance;

        expect(createTransition.prefundedVotingBalance.indexName).to.equal(newPrefundedVotingBalance.indexName);
        expect(createTransition.prefundedVotingBalance.credits).to.equal(newPrefundedVotingBalance.credits);
      });
    });

    describe('document Delete transition', () => {
      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const deleteTransition = new wasm.DocumentDeleteTransitionWASM(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        deleteTransition.base = newBase;

        expect(deleteTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });
    });

    describe('document Replace transition', () => {
      it('set data', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        const newData = { message: 'bebra' };

        replaceTransition.data = newData;

        expect(replaceTransition.data).to.deep.equal(newData);
      });

      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        replaceTransition.base = newBase;

        expect(replaceTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });

      it('set revision', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const replaceTransition = new wasm.DocumentReplaceTransitionWASM(documentInstance, BigInt(1));

        replaceTransition.revision = BigInt(11);

        expect(replaceTransition.revision).to.equal(BigInt(11));
      });
    });

    describe('document Transfer transition', () => {
      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        transferTransition.base = newBase;

        expect(transferTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });

      it('set recipient', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const transferTransition = new wasm.DocumentTransferTransitionWASM(documentInstance, BigInt(1), documentInstance.ownerId);

        const newRecipient = new Uint8Array(32);

        transferTransition.recipientId = newRecipient;

        expect(transferTransition.recipientId.bytes()).to.deep.equal(newRecipient);
      });
    });

    describe('document Update Price transition', () => {
      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        updatePriceTransition.base = newBase;

        expect(updatePriceTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });

      it('set price', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        updatePriceTransition.price = BigInt(1111);

        expect(updatePriceTransition.price).to.deep.equal(BigInt(1111));
      });
    });

    describe('document Purchase transition', () => {
      it('set base', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        const newBase = new wasm.DocumentBaseTransitionWASM(
          documentInstance.id,
          BigInt(12350),
          'bbbbb',
          dataContractId,
        );

        purchaseTransition.base = newBase;

        expect(purchaseTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase.__wbg_ptr).to.not.equal(0);
      });

      it('set price', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        purchaseTransition.price = BigInt(1111);

        expect(purchaseTransition.price).to.deep.equal(BigInt(1111));
      });

      it('set revision', () => {
        const documentInstance = new wasm.DocumentWASM(document, documentTypeName, revision, dataContractId, ownerId, id);
        const purchaseTransition = new wasm.DocumentPurchaseTransitionWASM(documentInstance, BigInt(1), BigInt(100));

        purchaseTransition.revision = BigInt(1111);

        expect(purchaseTransition.revision).to.deep.equal(BigInt(1111));
      });
    });
  });
});
