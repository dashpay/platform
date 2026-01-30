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

interface TokenBaseTransitionOptions {
  identityContractNonce?: bigint;
  tokenContractPosition?: number;
  dataContractId?: InstanceType<typeof wasm.Identifier>;
  tokenId?: InstanceType<typeof wasm.Identifier>;
}

describe('BatchTransition', () => {
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

  // Helper to create a token base transition with options object
  function createTokenBaseTransition(options: TokenBaseTransitionOptions = {}) {
    return new wasm.TokenBaseTransition({
      identityContractNonce: options.identityContractNonce ?? BigInt(1),
      tokenContractPosition: options.tokenContractPosition ?? 1,
      dataContractId: options.dataContractId ?? dataContractId,
      tokenId: options.tokenId ?? ownerId,
    });
  }

  describe('fromBatchedTransitions()', () => {
    describe('documents', () => {
      it('should create batch transition from document transitions', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

        const documentTransition = createTransition.toDocumentTransition();

        const batchedTransition = new wasm.BatchedTransition(documentTransition);

        const batch = wasm.BatchTransition.fromBatchedTransitions(
          [batchedTransition, batchedTransition],
          documentInstance.ownerId,
          1,
        );

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchedTransition).to.be.an.instanceof(wasm.BatchedTransition);
        expect(batch).to.be.an.instanceof(wasm.BatchTransition);
      });
    });

    describe('tokens', () => {
      it('should create batch transition from v1 transition', () => {
        const baseTransition = createTokenBaseTransition();

        const mintTransition = new wasm.TokenMintTransition({ base: baseTransition, issuedToIdentityId: ownerId, amount: BigInt(9999), publicNote: 'bbbbbb' });

        const transition = new wasm.TokenTransition(mintTransition);

        const batchedTransition = new wasm.BatchedTransition(transition);

        const batch = wasm.BatchTransition.fromBatchedTransitions(
          [batchedTransition, batchedTransition],
          ownerId,
          1,
        );

        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
        expect(mintTransition).to.be.an.instanceof(wasm.TokenMintTransition);
        expect(transition).to.be.an.instanceof(wasm.TokenTransition);
        expect(batchedTransition).to.be.an.instanceof(wasm.BatchedTransition);
        expect(batch).to.be.an.instanceof(wasm.BatchTransition);
      });
    });
  });

  describe('toBase64()', () => {
    it('should convert batch transition to base64', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batch = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition],
        documentInstance.ownerId,
        1,
      );

      const base64 = batch.toBase64();
      const bytes = batch.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('fromBase64()', () => {
    it('should create batch transition from base64', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batch = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition],
        documentInstance.ownerId,
        1,
      );

      const base64 = batch.toBase64();
      const bytes = batch.toBytes();

      const restoredBatch = wasm.BatchTransition.fromBase64(base64);

      expect(Buffer.from(restoredBatch.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject()', () => {
    it('should convert batch transition to object', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();
      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batch = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition],
        documentInstance.ownerId,
        1,
      );

      const object = batch.toObject();
      expect(object.signature).to.be.instanceOf(Uint8Array);
    });
  });

  describe('toJSON()', () => {
    it('should convert batch transition to JSON', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();
      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batch = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition],
        documentInstance.ownerId,
        1,
      );

      const json = batch.toJSON();
      expect(json.signature).to.be.a('string');
    });
  });

  describe('fromJSON()', () => {
    it('should create batch transition from JSON', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();
      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batch = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition],
        documentInstance.ownerId,
        1,
      );

      // Note: fromObject with complex nested structures containing Value fields
      // requires special handling due to serde_wasm_bindgen byte serialization.
      // Use fromJSON for reliable round-trip serialization.

      const json = batch.toJSON();

      const fromJson = wasm.BatchTransition.fromJSON(json);
      expect(Buffer.from(fromJson.toBytes())).to.deep.equal(Buffer.from(batch.toBytes()));
    });
  });

  describe('transitions', () => {
    it('should return transitions', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition, batchedTransition],
        documentInstance.ownerId,
        1,
      );

      expect(batchTransition.transitions.length).to.equal(2);
    });
  });

  describe('signature', () => {
    it('should return signature', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition, batchedTransition],
        documentInstance.ownerId,
        1,
      );

      expect(batchTransition.signature).to.deep.equal(new Uint8Array(0));
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should return signaturePublicKeyId', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [batchedTransition, batchedTransition],
        documentInstance.ownerId,
        1,
      );
      batchTransition.signaturePublicKeyId = 1;

      expect(batchTransition.signaturePublicKeyId).to.equal(1);
    });
  });

  describe('allPurchasesAmount', () => {
    it('should return allPurchasesAmount', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });
      const purchaseTransition = new wasm.DocumentPurchaseTransition({ document: documentInstance, identityContractNonce: BigInt(1), amount: BigInt(100) });

      const documentTransition = createTransition.toDocumentTransition();
      const documentTransition2 = purchaseTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition2)],
        documentInstance.ownerId,
        1,
      );

      expect(batchTransition.allPurchasesAmount).to.deep.equal(BigInt(100));
    });
  });

  describe('ownerId', () => {
    it('should return ownerId', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
        documentInstance.ownerId,
        1,
      );

      expect(batchTransition.ownerId.toBase58()).to.deep.equal(documentInstance.ownerId.toBase58());
    });
  });

  describe('modifiedDataIds', () => {
    it('should return modifiedDataIds', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
        documentInstance.ownerId,
        1,
      );

      const ids = batchTransition.modifiedDataIds.map(
        (identifier: InstanceType<typeof wasm.Identifier>) => identifier.toBase58(),
      );
      expect(ids).to.deep.equal([documentTransition.id.toBase58(), documentTransition.id.toBase58()]);
    });
  });

  describe('allConflictingIndexCollateralVotingFunds', () => {
    it('should return allConflictingIndexCollateralVotingFunds', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition({ document: documentInstance, identityContractNonce: BigInt(1) });

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions(
        [new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)],
        documentInstance.ownerId,
        1,
      );

      expect(batchTransition.allConflictingIndexCollateralVotingFunds).to.deep.equal(undefined);
    });
  });
});
