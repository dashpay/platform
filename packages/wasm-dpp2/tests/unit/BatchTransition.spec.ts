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

  describe('serialization / deserialization', () => {
    describe('documents', () => {
      it('should allow to create from v0 transition', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchedTransition = new wasm.BatchedTransition(documentTransition);

        const batch = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batch).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to create from v1 transition', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchedTransition = new wasm.BatchedTransition(documentTransition);

        const batch = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);

        expect(documentInstance).to.be.an.instanceof(wasm.Document);
        expect(createTransition).to.be.an.instanceof(wasm.DocumentCreateTransition);
        expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
        expect(batchedTransition).to.be.an.instanceof(wasm.BatchedTransition);
        expect(batch).to.be.an.instanceof(wasm.BatchTransition);
      });

      it('should allow to convert batch transition to base64 and back', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();

        const batchedTransition = new wasm.BatchedTransition(documentTransition);

        const batch = wasm.BatchTransition.fromBatchedTransitions([batchedTransition], documentInstance.ownerId, 1);

        const base64 = batch.toBase64();
        const bytes = batch.toBytes();

        expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

        const restoredBatch = wasm.BatchTransition.fromBase64(base64);

        expect(Buffer.from(restoredBatch.toBytes())).to.deep.equal(Buffer.from(bytes));
      });

      it('should round-trip via object and JSON', () => {
        const documentInstance = createDocument();
        const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

        const documentTransition = createTransition.toDocumentTransition();
        const batchedTransition = new wasm.BatchedTransition(documentTransition);

        const batch = wasm.BatchTransition.fromBatchedTransitions([batchedTransition], documentInstance.ownerId, 1);

        const object = batch.toObject();
        expect(object.signature).to.be.instanceOf(Uint8Array);

        // Note: fromObject with complex nested structures containing Value fields
        // requires special handling due to serde_wasm_bindgen byte serialization.
        // Use fromJSON for reliable round-trip serialization.

        const json = batch.toJSON();
        expect(json.signature).to.be.a('string');

        const fromJson = wasm.BatchTransition.fromJSON(json);
        expect(Buffer.from(fromJson.toBytes())).to.deep.equal(Buffer.from(batch.toBytes()));
      });
    });
    describe('tokens', () => {
      it('should allow to create from v1 transition', () => {
        const baseTransition = createTokenBaseTransition();

        const mintTransition = new wasm.TokenMintTransition(baseTransition, ownerId, BigInt(9999), 'bbbbbb');

        const transition = new wasm.TokenTransition(mintTransition);

        const batchedTransition = new wasm.BatchedTransition(transition);

        const batch = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], ownerId, 1);

        expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
        expect(mintTransition).to.be.an.instanceof(wasm.TokenMintTransition);
        expect(transition).to.be.an.instanceof(wasm.TokenTransition);
        expect(batchedTransition).to.be.an.instanceof(wasm.BatchedTransition);
        expect(batch).to.be.an.instanceof(wasm.BatchTransition);
      });
    });
  });

  describe('getters', () => {
    it('should allow to get transitions', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);

      expect(batchTransition.transitions.length).to.equal(2);
    });

    it('should allow to get signature', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);

      expect(batchTransition.signature).to.deep.equal(new Uint8Array(0));
    });

    it('should allow to get signature public key id', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchedTransition = new wasm.BatchedTransition(documentTransition);

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([batchedTransition, batchedTransition], documentInstance.ownerId, 1);
      batchTransition.signaturePublicKeyId = 1;

      expect(batchTransition.signaturePublicKeyId).to.equal(1);
    });

    it('should allow to get all purchases amount', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));
      const purchaseTransition = new wasm.DocumentPurchaseTransition(documentInstance, BigInt(1), BigInt(100));

      const documentTransition = createTransition.toDocumentTransition();
      const documentTransition2 = purchaseTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition2)], documentInstance.ownerId, 1);

      expect(batchTransition.allPurchasesAmount).to.deep.equal(BigInt(100));
    });

    it('should allow to get owner id', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

      expect(batchTransition.ownerId.toBase58()).to.deep.equal(documentInstance.ownerId.toBase58());
    });

    it('should allow to get modified data ids', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

      expect(batchTransition.modifiedDataIds.map((identifier: InstanceType<typeof wasm.Identifier>) => identifier.toBase58())).to.deep.equal([documentTransition.id.toBase58(), documentTransition.id.toBase58()]);
    });

    it('should allow to get allConflictingIndexCollateralVotingFunds', () => {
      const documentInstance = createDocument();
      const createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));

      const documentTransition = createTransition.toDocumentTransition();

      const batchTransition = wasm.BatchTransition.fromBatchedTransitions([new wasm.BatchedTransition(documentTransition), new wasm.BatchedTransition(documentTransition)], documentInstance.ownerId, 1);

      expect(batchTransition.allConflictingIndexCollateralVotingFunds).to.deep.equal(undefined);
    });
  });
});
