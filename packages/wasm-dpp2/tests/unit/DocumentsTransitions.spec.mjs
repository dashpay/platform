import getWasm from './helpers/wasm.js';
import {
  document, documentTypeName, revision, dataContractId, ownerId, id,
} from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('DocumentsTransitions', () => {
  // Helper to create a document with options object
  function createDocument(options = {}) {
    return new wasm.Document({
      properties: options.properties ?? document,
      documentTypeName: options.documentTypeName ?? documentTypeName,
      dataContractId: options.dataContractId ?? dataContractId,
      ownerId: options.ownerId ?? ownerId,
      revision: options.revision ?? BigInt(revision),
      id: options.id ?? id,
    });
  }

  describe('serialization / deserialization', () => {
    describe('document Create transition', () => {
      it('should allow to create CreateTransition from document', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
      });

      it('should allow to create Document Transition from Create transition', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedCreateTransition = deserializedTransitions[0].toTransition().createTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedCreateTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
      });
    });

    describe('document Delete transition', () => {
      it('should allow to create DeleteTransition from document', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(deleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
      });

      it('should allow to create Document Transition from Delete transition', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(deleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(deleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        const documentTransition = deleteTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedDeleteTransition = deserializedTransitions[0].toTransition().deleteTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(deleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedDeleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
      });
    });

    describe('document Replace transition', () => {
      it('should allow to create ReplaceTransition from document', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(replaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
      });

      it('should allow to create Document Transition from Replace transition', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(replaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(replaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        const documentTransition = replaceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedReplaceTransition = deserializedTransitions[0].toTransition().replaceTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(replaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedReplaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
      });
    });

    describe('document Transfer transition', () => {
      it('should allow to create ReplaceTransition from document', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(transferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
      });

      it('should allow to create Document Transition from Replace transition', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(transferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(transferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        const documentTransition = transferTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedTransferTransition = deserializedTransitions[0].toTransition().transferTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(transferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedTransferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
      });
    });

    describe('document UpdatePrice transition', () => {
      it('should allow to create UpdatePriceTransition from document', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(updatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
      });

      it('should allow to create Document Transition from UpdatePrice transition', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(updatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(updatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = updatePriceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedUpdatePriceTransition = deserializedTransitions[0].toTransition().updatePriceTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(updatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedUpdatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
      });
    });

    describe('document Purchase transition', () => {
      it('should allow to create PurchaseTransition from document', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(purchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
      });

      it('should allow to create Document Transition from PurchaseTransition transition', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(purchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });

      it('should allow to create Document Batch Transition from Document Transitions', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(purchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create state document_transitions from document and convert state transition to document batch', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        const documentTransition = purchaseTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().purchaseTransition;

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(purchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
        expect(st).to.be.an.instanceof(wasm.StateTransition);
        expect(deserializedBatch).to.be.an.instanceof(wasm.BatchTransition);
        expect(deserializedPurchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
      });
    });
  });
  describe('getters', () => {
    describe('document Create transition', () => {
      it('get data', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        expect(createTransition.data).to.deep.equal(document);
      });

      it('get base', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        expect(createTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('get entropy', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        expect(createTransition.entropy).to.deep.equal(documentInstance.entropy);
      });

      it('get prefunded voting balance', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        expect(createTransition.prefundedVotingBalance).to.equal(undefined);
      });
    });

    describe('document Delete transition', () => {
      it('get base', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        expect(deleteTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });
    });

    describe('document Replace transition', () => {
      it('get data', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        expect(replaceTransition.data).to.deep.equal(document);
      });

      it('get base', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        expect(replaceTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('get revision', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        expect(replaceTransition.revision).to.equal(BigInt(2));
      });
    });

    describe('document Transfer transition', () => {
      it('get base', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(transferTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('get recipient', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        expect(transferTransition.recipientOwnerId.toBase58()).to.deep.equal(documentInstance.ownerId.toBase58());
      });
    });

    describe('document Update Price transition', () => {
      it('get base', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        expect(updatePriceTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('get price', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        expect(updatePriceTransition.price).to.deep.equal(BigInt(100));
      });
    });

    describe('document Purchase transition', () => {
      it('get base', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        expect(purchaseTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('get price', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        expect(purchaseTransition.price).to.deep.equal(BigInt(100));
      });
    });
  });

  describe('setters', () => {
    describe('document Create transition', () => {
      it('set data', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const newData = { message: 'bebra' };

        createTransition.data = newData;

        expect(createTransition.data).to.deep.equal(newData);
      });

      it('set base', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        createTransition.base = newBase;

        expect(createTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });

      it('set entropy', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const newEntropy = new Uint8Array(32);

        createTransition.entropy = newEntropy;

        expect(createTransition.entropy).to.deep.equal(newEntropy);
      });

      it('set prefunded voting balance', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const newPrefundedVotingBalance = new wasm.PrefundedVotingBalance('note', BigInt(9999));

        createTransition.prefundedVotingBalance = newPrefundedVotingBalance;

        expect(createTransition.prefundedVotingBalance.indexName).to.equal(newPrefundedVotingBalance.indexName);
        expect(createTransition.prefundedVotingBalance.credits).to.equal(newPrefundedVotingBalance.credits);
      });
    });

    describe('document Delete transition', () => {
      it('set base', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        deleteTransition.base = newBase;

        expect(deleteTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });
    });

    describe('document Replace transition', () => {
      it('set data', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        const newData = { message: 'bebra' };

        replaceTransition.data = newData;

        expect(replaceTransition.data).to.deep.equal(newData);
      });

      it('set base', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        replaceTransition.base = newBase;

        expect(replaceTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });

      it('set revision', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));

        replaceTransition.revision = BigInt(11);

        expect(replaceTransition.revision).to.equal(BigInt(11));
      });
    });

    describe('document Transfer transition', () => {
      it('set base', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        transferTransition.base = newBase;

        expect(transferTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });

      it('set recipient', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition(documentInstance, BigInt(1), documentInstance.ownerId);

        const newRecipient = new Uint8Array(32);

        transferTransition.recipientOwnerId = newRecipient;

        expect(transferTransition.recipientOwnerId.toBytes()).to.deep.equal(newRecipient);
      });
    });

    describe('document Update Price transition', () => {
      it('set base', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        updatePriceTransition.base = newBase;

        expect(updatePriceTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });

      it('set price', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition(documentInstance, BigInt(1), BigInt(100));

        updatePriceTransition.price = BigInt(1111);

        expect(updatePriceTransition.price).to.deep.equal(BigInt(1111));
      });
    });

    describe('document Purchase transition', () => {
      it('set base', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        const newBase = new wasm.DocumentBaseTransition({
          documentId: documentInstance.id,
          identityContractNonce: BigInt(12350),
          documentTypeName: 'bbbbb',
          dataContractId,
        });

        purchaseTransition.base = newBase;

        expect(purchaseTransition.base.identityContractNonce).to.equal(newBase.identityContractNonce);
        expect(newBase).to.be.an.instanceof(wasm.DocumentBaseTransition);
      });

      it('set price', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        purchaseTransition.price = BigInt(1111);

        expect(purchaseTransition.price).to.deep.equal(BigInt(1111));
      });

      it('set revision', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

        purchaseTransition.revision = BigInt(1111);

        expect(purchaseTransition.revision).to.deep.equal(BigInt(1111));
      });
    });
  });
});
