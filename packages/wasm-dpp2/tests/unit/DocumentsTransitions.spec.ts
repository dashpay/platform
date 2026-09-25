import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  document, documentTypeName, revision, dataContractId, ownerId, id,
} from './mocks/Document/index.js';

before(async () => {
  await initWasm();
});

interface DocumentOptions {
  properties?: Record<string, unknown>;
  documentTypeName?: string;
  dataContractId?: InstanceType<typeof wasm.Identifier>;
  ownerId?: InstanceType<typeof wasm.Identifier>;
  revision?: bigint;
  id?: InstanceType<typeof wasm.Identifier>;
}

describe('DocumentsTransitions', () => {
  // Helper to create a document with options object
  function createDocument(options: DocumentOptions = {}) {
    return new wasm.Document({
      properties: options.properties ?? document,
      documentTypeName: options.documentTypeName ?? documentTypeName,
      dataContractId: options.dataContractId ?? dataContractId,
      ownerId: options.ownerId ?? ownerId,
      revision: options.revision ?? BigInt(revision),
      id: options.id ?? id,
    });
  }

  describe('DocumentBaseTransition', () => {
    function createAgreement() {
      return new wasm.DocumentActionFeeAgreement({
        owner: 10000000n,
        moderators: 100000000n,
        feeMultiplier: {
          knownPermille: 1000n,
          increaseTolerancePercent: 20,
        },
      });
    }

    function createBase(actionFeeAgreement?: InstanceType<typeof wasm.DocumentActionFeeAgreement>) {
      const documentInstance = createDocument();

      return new wasm.DocumentBaseTransition({
        documentId: documentInstance.id,
        identityContractNonce: BigInt(1),
        documentTypeName,
        dataContractId,
        actionFeeAgreement,
      });
    }

    it('should carry the action fee agreement it was created with', () => {
      const base = createBase(createAgreement());

      const agreement = base.actionFeeAgreement;

      expect(agreement.owner).to.equal(10000000n);
      expect(agreement.moderators).to.equal(100000000n);
      expect(agreement.knownFeeMultiplierPermille).to.equal(1000n);
      expect(agreement.feeMultiplierIncreaseTolerancePercent).to.equal(20);
      expect(agreement.pricing).to.equal('feeMultiplier');
    });

    it('should have no action fee agreement when none is given', () => {
      const base = createBase();

      expect(base.actionFeeAgreement).to.be.undefined();
    });

    it('should set the action fee agreement', () => {
      const base = createBase();

      base.actionFeeAgreement = createAgreement();

      expect(base.actionFeeAgreement.owner).to.equal(10000000n);
    });

    it('should clear the action fee agreement with undefined', () => {
      const base = createBase(createAgreement());

      base.actionFeeAgreement = undefined;

      expect(base.actionFeeAgreement).to.be.undefined();
    });
  });

  describe('DocumentCreateTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
      });

      it('should derive the id from the entropy and the nonce and mirror it onto the document', () => {
        // the document is built with an id that is not the one its create
        // transition must carry: the constructor replaces it
        const documentInstance = createDocument();
        expect(documentInstance.id.toBase58()).to.equal(id);

        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(7),
        });

        const derived = wasm.Document.generateId(
          documentTypeName,
          ownerId,
          dataContractId,
          documentInstance.entropy,
          BigInt(7),
        );

        expect(createTransition.base.id.toBytes()).to.deep.equal(derived);
        expect(documentInstance.id.toBytes()).to.deep.equal(derived);
        expect(documentInstance.id.toBase58()).to.not.equal(id);
      });

      it('should derive a different id for another nonce', () => {
        const documentInstance = createDocument();
        const { entropy } = documentInstance;

        const first = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });
        const second = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(2),
        });

        expect(first.entropy).to.deep.equal(entropy);
        expect(second.entropy).to.deep.equal(entropy);
        expect(first.base.id.toBase58()).to.not.equal(second.base.id.toBase58());
        // the document follows the transition it was last built into
        expect(documentInstance.id.toBase58()).to.equal(second.base.id.toBase58());
      });

      it('should keep the entropy-only id before protocol version 14', () => {
        const documentInstance = new wasm.Document({
          properties: document,
          documentTypeName,
          dataContractId,
          ownerId,
          revision: BigInt(revision),
        });
        const placeholder = documentInstance.id.toBase58();

        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          platformVersion: 13,
        });

        expect(createTransition.base.id.toBase58()).to.equal(placeholder);
        expect(documentInstance.id.toBase58()).to.equal(placeholder);
      });

      it('should refuse a document without entropy', () => {
        // a document read back from Platform carries none: it exists already
        const documentInstance = createDocument();
        documentInstance.entropy = undefined;

        expect(() => new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        })).to.throw(/entropy/);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from CreateTransition', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = createTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('data', () => {
      it('should return data', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(createTransition.data).to.deep.equal(document);
      });

      it('should set data', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const newData = { message: 'bebra' };

        createTransition.data = newData;

        expect(createTransition.data).to.deep.equal(newData);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(createTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

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
    });

    describe('actionFeeAgreement', () => {
      it('should pass the action fee agreement to the base', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          actionFeeAgreement: new wasm.DocumentActionFeeAgreement({
            owner: 5000n,
            moderators: 7000n,
          }),
        });

        const agreement = createTransition.base.actionFeeAgreement;

        expect(agreement.owner).to.equal(5000n);
        expect(agreement.moderators).to.equal(7000n);
        expect(agreement.pricing).to.equal('fixed');
      });

      it('should leave the base without an agreement when none is given', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(createTransition.base.actionFeeAgreement).to.be.undefined();
      });
    });

    describe('entropy', () => {
      it('should return entropy', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(createTransition.entropy).to.deep.equal(documentInstance.entropy);
      });

      it('should set entropy', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const newEntropy = new Uint8Array(32);

        createTransition.entropy = newEntropy;

        expect(createTransition.entropy).to.deep.equal(newEntropy);
      });
    });

    describe('prefundedVotingBalance', () => {
      it('should return prefunded voting balance', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(createTransition.prefundedVotingBalance).to.equal(undefined);
      });

      it('should set prefunded voting balance', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const newPrefundedVotingBalance = new wasm.PrefundedVotingBalance({ indexName: 'note', credits: BigInt(9999) });

        createTransition.prefundedVotingBalance = newPrefundedVotingBalance;

        expect(createTransition.prefundedVotingBalance.indexName).to.equal(newPrefundedVotingBalance.indexName);
        expect(createTransition.prefundedVotingBalance.credits).to.equal(newPrefundedVotingBalance.credits);
      });
    });

    describe('BatchTransition.fromBatchedTransitions()', () => {
      it('should create BatchTransition from document transitions', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        expect(batchTransition).to.be.an.instanceof(wasm.BatchTransition);
      });
    });

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = createTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedCreateTransition = deserializedTransitions[0].toTransition().createTransition;

        expect(deserializedCreateTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
      });
    });
  });

  describe('DocumentDeleteTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(deleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from DeleteTransition', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = deleteTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(deleteTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

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

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const deleteTransition = new wasm.DocumentDeleteTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = deleteTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedDeleteTransition = deserializedTransitions[0].toTransition().deleteTransition;

        expect(deserializedDeleteTransition).to.be.an.instanceof(wasm.DocumentDeleteTransition);
      });
    });
  });

  describe('DocumentReplaceTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(replaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from ReplaceTransition', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = replaceTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('data', () => {
      it('should return data', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(replaceTransition.data).to.deep.equal(document);
      });

      it('should set data', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const newData = { message: 'bebra' };

        replaceTransition.data = newData;

        expect(replaceTransition.data).to.deep.equal(newData);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(replaceTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

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
    });

    describe('revision', () => {
      it('should return revision', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        expect(replaceTransition.revision).to.equal(BigInt(2));
      });

      it('should set revision', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        replaceTransition.revision = BigInt(11);

        expect(replaceTransition.revision).to.equal(BigInt(11));
      });
    });

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const replaceTransition = new wasm.DocumentReplaceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
        });

        const documentTransition = replaceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedReplaceTransition = deserializedTransitions[0].toTransition().replaceTransition;

        expect(deserializedReplaceTransition).to.be.an.instanceof(wasm.DocumentReplaceTransition);
      });
    });
  });

  describe('DocumentTransferTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(transferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from TransferTransition', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        const documentTransition = transferTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        expect(transferTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

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
    });

    describe('recipientOwnerId', () => {
      it('should return recipient', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        expect(transferTransition.recipientOwnerId.toBase58()).to.deep.equal(documentInstance.ownerId.toBase58());
      });

      it('should set recipient', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        const newRecipient = new Uint8Array(32);

        transferTransition.recipientOwnerId = newRecipient;

        expect(transferTransition.recipientOwnerId.toBytes()).to.deep.equal(newRecipient);
      });
    });

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const transferTransition = new wasm.DocumentTransferTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          recipientOwnerId: documentInstance.ownerId,
        });

        const documentTransition = transferTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedTransferTransition = deserializedTransitions[0].toTransition().transferTransition;

        expect(deserializedTransferTransition).to.be.an.instanceof(wasm.DocumentTransferTransition);
      });
    });
  });

  describe('DocumentUpdatePriceTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(updatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from UpdatePriceTransition', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        const documentTransition = updatePriceTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        expect(updatePriceTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

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
    });

    describe('price', () => {
      it('should return price', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        expect(updatePriceTransition.price).to.deep.equal(BigInt(100));
      });

      it('should set price', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        updatePriceTransition.price = BigInt(1111);

        expect(updatePriceTransition.price).to.deep.equal(BigInt(1111));
      });
    });

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const updatePriceTransition = new wasm.DocumentUpdatePriceTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          price: BigInt(100),
        });

        const documentTransition = updatePriceTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedUpdatePriceTransition = deserializedTransitions[0].toTransition().updatePriceTransition;

        expect(deserializedUpdatePriceTransition).to.be.an.instanceof(wasm.DocumentUpdatePriceTransition);
      });
    });
  });

  describe('DocumentPurchaseTransition', () => {
    describe('constructor', () => {
      it('should create instance from document', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(purchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
      });
    });

    describe('toDocumentTransition()', () => {
      it('should create DocumentTransition from PurchaseTransition', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        const documentTransition = purchaseTransition.toDocumentTransition();

        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
      });
    });

    describe('base', () => {
      it('should return base', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        expect(purchaseTransition.base.constructor.name).to.equal('DocumentBaseTransition');
      });

      it('should set base', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

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
    });

    describe('price', () => {
      it('should return price', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        expect(purchaseTransition.price).to.deep.equal(BigInt(100));
      });

      it('should set price', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        purchaseTransition.price = BigInt(1111);

        expect(purchaseTransition.price).to.deep.equal(BigInt(1111));
      });
    });

    describe('revision', () => {
      it('should set revision', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        purchaseTransition.revision = BigInt(1111);

        expect(purchaseTransition.revision).to.deep.equal(BigInt(1111));
      });
    });

    describe('BatchTransition serialization roundtrip', () => {
      it('should serialize and deserialize through state transition', () => {
        const documentInstance = createDocument();
        const purchaseTransition = new wasm.DocumentPurchaseTransition({
          document: documentInstance,
          identityContractNonce: BigInt(1),
          amount: BigInt(100),
        });

        const documentTransition = purchaseTransition.toDocumentTransition();

        const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
          [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
          documentInstance.ownerId,
          1,
        );

        const st = batchTransition.toStateTransition();

        const deserializedBatch = wasm.BatchTransition.fromStateTransition(st);

        const deserializedTransitions = deserializedBatch.transitions;

        expect(deserializedTransitions.length).to.equal(2);

        const deserializedPurchaseTransition = deserializedTransitions[0].toTransition().purchaseTransition;

        expect(deserializedPurchaseTransition).to.be.an.instanceof(wasm.DocumentPurchaseTransition);
      });
    });
  });
});
