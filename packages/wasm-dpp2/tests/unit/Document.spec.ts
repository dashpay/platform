import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  document,
  dataContractId,
  ownerId,
  documentTypeName,
  revision,
  dataContractValue,
  id,
  document2,
  documentBytes,
} from './mocks/Document/index.js';
import { fromHexString } from './utils/hex.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

describe('Document', () => {
  // Helper to create a document with options object
  function createDocument(
    options: {
      properties?: Record<string, unknown>;
      documentTypeName?: string;
      dataContractId?: string;
      ownerId?: string;
      revision?: bigint;
      id?: string;
      entropy?: Uint8Array;
    } = {},
  ) {
    return new wasm.Document({
      properties: options.properties ?? document,
      documentTypeName: options.documentTypeName ?? documentTypeName,
      dataContractId: options.dataContractId ?? dataContractId,
      ownerId: options.ownerId ?? ownerId,
      revision: options.revision ?? BigInt(revision),
      id: options.id,
      entropy: options.entropy,
    });
  }

  describe('serialization / deserialization', () => {
    it('should allows to create Document from values', () => {
      const documentInstance = createDocument();

      expect(documentInstance).to.be.an.instanceof(wasm.Document);
    });

    it('should allows to create Document from values with custom id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance).to.be.an.instanceof(wasm.Document);
    });

    it('should allows to create Document from bytes and convert to bytes', () => {
      const dataContract = wasm.DataContract.fromJSON(dataContractValue, false);
      const documentInstance = wasm.Document.fromBytes(
        fromHexString(documentBytes),
        dataContract,
        'note',
        PlatformVersion.PLATFORM_V1,
      );

      const bytes = documentInstance.toBytes(dataContract, PlatformVersion.PLATFORM_V1);

      expect(documentInstance.dataContractId.toBase58()).to.equal(dataContract.id.toBase58());
      expect(bytes).to.deep.equal(fromHexString(documentBytes));
      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('toObject / fromObject', () => {
    it('should convert to object with binary fields as Uint8Array', () => {
      const documentInstance = createDocument({ id });

      const obj = documentInstance.toObject();

      expect(obj.$id).to.be.instanceOf(Uint8Array);
      expect(obj.$ownerId).to.be.instanceOf(Uint8Array);
      expect(obj.$dataContractId).to.be.instanceOf(Uint8Array);
      expect(obj.$type).to.equal(documentTypeName);
      // toObject uses BigInt for u64 values like revision to preserve precision
      expect(BigInt(obj.$revision)).to.equal(revision);
    });

    it('should roundtrip through toObject / fromObject', () => {
      const documentInstance = createDocument({ id });

      const obj = documentInstance.toObject();
      const restored = wasm.Document.fromObject(obj);

      expect(restored.id.toBase58()).to.equal(documentInstance.id.toBase58());
      expect(restored.ownerId.toBase58()).to.equal(documentInstance.ownerId.toBase58());
      expect(restored.dataContractId.toBase58()).to.equal(documentInstance.dataContractId.toBase58());
      expect(restored.documentTypeName).to.equal(documentInstance.documentTypeName);
      expect(restored.revision).to.equal(documentInstance.revision);
    });
  });

  describe('toJSON / fromJSON', () => {
    it('should convert to JSON with identifiers as Base58 strings', () => {
      const documentInstance = createDocument({ id });

      const json = documentInstance.toJSON();

      expect(typeof json.$id).to.equal('string');
      expect(typeof json.$ownerId).to.equal('string');
      expect(typeof json.$dataContractId).to.equal('string');
      expect(json.$type).to.equal(documentTypeName);
      expect(json.$revision).to.equal(Number(revision));
    });

    it('should roundtrip through toJSON / fromJSON', () => {
      const documentInstance = createDocument({ id });

      const json = documentInstance.toJSON();
      const restored = wasm.Document.fromJSON(json);

      expect(restored.id.toBase58()).to.equal(documentInstance.id.toBase58());
      expect(restored.ownerId.toBase58()).to.equal(documentInstance.ownerId.toBase58());
      expect(restored.dataContractId.toBase58()).to.equal(documentInstance.dataContractId.toBase58());
      expect(restored.documentTypeName).to.equal(documentInstance.documentTypeName);
      expect(restored.revision).to.equal(documentInstance.revision);
    });
  });

  describe('getters', () => {
    it('should return document id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.id.toBase58()).to.deep.equal(id);
    });

    it('should return owner id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.ownerId.toBase58()).to.deep.equal(ownerId);
    });

    it('should return data contract id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.dataContractId.toBase58()).to.deep.equal(dataContractId);
    });

    it('should return properties', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.properties).to.deep.equal(document);
    });

    it('should return revision', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.revision).to.deep.equal(revision);
    });
  });

  describe('setters', () => {
    it('should allow to set document id', () => {
      const documentInstance = createDocument({ id });

      documentInstance.id = ownerId;

      expect(documentInstance.id.toBase58()).to.deep.equal(ownerId);
    });

    it('should allow to set document owner id', () => {
      const documentInstance = createDocument({ id });

      documentInstance.ownerId = id;

      expect(documentInstance.ownerId.toBase58()).to.deep.equal(id);
    });

    it('should allow to set entropy', () => {
      const documentInstance = createDocument({ id });

      const newEntropy = new Array(documentInstance.entropy.length).fill(0);

      documentInstance.entropy = newEntropy;

      expect(Array.from(documentInstance.entropy)).to.deep.equal(newEntropy);
    });

    it('should allow to set properties', () => {
      const documentInstance = createDocument({ id });

      documentInstance.properties = document2;

      expect(documentInstance.properties).to.deep.equal(document2);
    });

    it('should allow to set revision', () => {
      const documentInstance = createDocument({ id });

      const newRevision = BigInt(1000);

      documentInstance.revision = newRevision;

      expect(documentInstance.revision).to.deep.equal(newRevision);
    });

    it('should allow to set created at', () => {
      const documentInstance = createDocument({ id });

      const createdAt = BigInt(new Date(1123).getTime());

      documentInstance.createdAt = createdAt;

      expect(documentInstance.createdAt).to.deep.equal(createdAt);
    });

    it('should allow to set updated at', () => {
      const documentInstance = createDocument({ id });

      const updatedAt = BigInt(new Date(1123).getTime());

      documentInstance.updatedAt = updatedAt;

      expect(documentInstance.updatedAt).to.deep.equal(updatedAt);
    });

    it('should allow to set transferred at', () => {
      const documentInstance = createDocument({ id });

      const transferredAt = BigInt(new Date(11231).getTime());

      documentInstance.transferredAt = transferredAt;

      expect(documentInstance.transferredAt).to.deep.equal(transferredAt);
    });

    it('should allow to set create at Block Height', () => {
      const documentInstance = createDocument({ id });

      const createdAtHeight = BigInt(9172);

      documentInstance.createdAtBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtBlockHeight).to.deep.equal(createdAtHeight);
    });

    it('should allow to set updated at Block Height', () => {
      const documentInstance = createDocument({ id });

      const updatedAtHeight = BigInt(9172);

      documentInstance.updatedAtBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtBlockHeight).to.deep.equal(updatedAtHeight);
    });

    it('should allow to set transferred at Block Height', () => {
      const documentInstance = createDocument({ id });

      const transferredAtHeight = BigInt(9172);

      documentInstance.transferredAtBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtBlockHeight).to.deep.equal(transferredAtHeight);
    });

    it('should allow to set create at core Block Height', () => {
      const documentInstance = createDocument({ id });

      const createdAtHeight = 91721;

      documentInstance.createdAtCoreBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtCoreBlockHeight).to.deep.equal(createdAtHeight);
    });

    it('should allow to set updated at Block Height', () => {
      const documentInstance = createDocument({ id });

      const updatedAtHeight = 91722;

      documentInstance.updatedAtCoreBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtCoreBlockHeight).to.deep.equal(updatedAtHeight);
    });

    it('should allow to set transferred at Block Height', () => {
      const documentInstance = createDocument({ id });

      const transferredAtHeight = 91723;

      documentInstance.transferredAtCoreBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtCoreBlockHeight).to.deep.equal(transferredAtHeight);
    });

    it('should allow to set document type name', () => {
      const documentInstance = createDocument({ id });

      const newDocumentTypeName = 'bbbb';

      documentInstance.documentTypeName = newDocumentTypeName;

      expect(documentInstance.documentTypeName).to.deep.equal(newDocumentTypeName);
    });
  });

  describe('static', () => {
    it('should allow to generate id', () => {
      const generatedId = wasm.Document.generateId('note', ownerId, dataContractId);

      expect(Array.from(generatedId).length).to.equal(32);
    });
  });
});
