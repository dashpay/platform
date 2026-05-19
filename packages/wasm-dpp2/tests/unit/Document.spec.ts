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
import { fromHexString } from './utils/hex.ts';

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

  // Fixed 32-byte entropy for deterministic tests
  const fixedEntropy = Uint8Array.from([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
  ]);

  // Pre-computed byte arrays for identifiers (from Base58 decoding)
  const idBytes = Uint8Array.from([
    132, 11, 92, 236, 159, 4, 1, 177, 149, 6, 16, 232, 93, 203, 38, 130,
    145, 34, 172, 136, 83, 70, 78, 30, 44, 126, 56, 237, 34, 180, 137, 3,
  ]);
  const ownerIdBytes = Uint8Array.from([
    171, 50, 27, 25, 69, 123, 28, 22, 177, 39, 98, 80, 146, 118, 212, 125,
    149, 47, 88, 45, 151, 140, 189, 139, 80, 187, 252, 47, 216, 167, 118, 125,
  ]);
  const dataContractIdBytes = Uint8Array.from([
    234, 137, 32, 237, 234, 82, 250, 64, 13, 226, 204, 196, 48, 82, 178, 130,
    31, 69, 88, 82, 65, 162, 135, 93, 66, 80, 227, 93, 76, 233, 55, 200,
  ]);

  describe('constructor()', () => {
    it('should create Document from values', () => {
      const documentInstance = createDocument();

      expect(documentInstance).to.be.an.instanceof(wasm.Document);
    });

    it('should create Document from values with custom id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance).to.be.an.instanceof(wasm.Document);
    });
  });

  describe('toBytes()', () => {
    it('should convert Document to bytes', () => {
      const dataContract = wasm.DataContract.fromJSON(dataContractValue, false);
      const documentInstance = wasm.Document.fromBytes(
        fromHexString(documentBytes),
        dataContract,
        'note',
        new PlatformVersion(1),
      );

      const bytes = documentInstance.toBytes(dataContract, new PlatformVersion(1));

      expect(bytes).to.deep.equal(fromHexString(documentBytes));
    });
  });

  describe('fromBytes()', () => {
    it('should create Document from bytes', () => {
      const dataContract = wasm.DataContract.fromJSON(dataContractValue, false);
      const documentInstance = wasm.Document.fromBytes(
        fromHexString(documentBytes),
        dataContract,
        'note',
        new PlatformVersion(1),
      );

      expect(documentInstance.dataContractId.toBase58()).to.equal(dataContract.id.toBase58());
      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('toObject()', () => {
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

    it('should match expected byte values for known input', () => {
      const doc = createDocument({ id, entropy: fixedEntropy });
      const obj = doc.toObject();

      expect(obj.$id).to.deep.equal(idBytes);
      expect(obj.$ownerId).to.deep.equal(ownerIdBytes);
      expect(obj.$dataContractId).to.deep.equal(dataContractIdBytes);
      expect(obj.$type).to.equal(documentTypeName);
      expect(obj.$revision).to.equal(1n);
      expect(obj.$entropy).to.deep.equal(fixedEntropy);
      expect(obj.message).to.equal('Tutorial CI Test @ Tue, 07 Jan 2025 15:27:50 GMT');
    });
  });

  describe('fromObject()', () => {
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

  describe('toJSON()', () => {
    it('should convert to JSON with identifiers as Base58 strings', () => {
      const documentInstance = createDocument({ id });

      const json = documentInstance.toJSON();

      expect(typeof json.$id).to.equal('string');
      expect(typeof json.$ownerId).to.equal('string');
      expect(typeof json.$dataContractId).to.equal('string');
      expect(json.$type).to.equal(documentTypeName);
      expect(json.$revision).to.equal(Number(revision));
    });

    it('should match expected values for known input', () => {
      const doc = createDocument({ id, entropy: fixedEntropy });
      const json = doc.toJSON();

      expect(json.$id).to.equal(id);
      expect(json.$ownerId).to.equal(ownerId);
      expect(json.$dataContractId).to.equal(dataContractId);
      expect(json.$type).to.equal(documentTypeName);
      expect(json.$revision).to.equal(Number(revision));
      expect(json.message).to.equal('Tutorial CI Test @ Tue, 07 Jan 2025 15:27:50 GMT');
      expect(typeof json.$entropy).to.equal('string');
    });
  });

  describe('fromJSON()', () => {
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

  describe('generateId()', () => {
    it('should generate id', () => {
      const generatedId = wasm.Document.generateId('note', ownerId, dataContractId);

      expect(Array.from(generatedId).length).to.equal(32);
    });
  });

  describe('id', () => {
    it('should return document id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.id.toBase58()).to.deep.equal(id);
    });

    it('should set document id', () => {
      const documentInstance = createDocument({ id });

      documentInstance.id = ownerId;

      expect(documentInstance.id.toBase58()).to.deep.equal(ownerId);
    });
  });

  describe('ownerId', () => {
    it('should return owner id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.ownerId.toBase58()).to.deep.equal(ownerId);
    });

    it('should set document owner id', () => {
      const documentInstance = createDocument({ id });

      documentInstance.ownerId = id;

      expect(documentInstance.ownerId.toBase58()).to.deep.equal(id);
    });
  });

  describe('dataContractId', () => {
    it('should return data contract id', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.dataContractId.toBase58()).to.deep.equal(dataContractId);
    });
  });

  describe('properties', () => {
    it('should return properties', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.properties).to.deep.equal(document);
    });

    it('should set properties', () => {
      const documentInstance = createDocument({ id });

      documentInstance.properties = document2;

      expect(documentInstance.properties).to.deep.equal(document2);
    });
  });

  describe('revision', () => {
    it('should return revision', () => {
      const documentInstance = createDocument({ id });

      expect(documentInstance.revision).to.deep.equal(revision);
    });

    it('should set revision', () => {
      const documentInstance = createDocument({ id });

      const newRevision = BigInt(1000);

      documentInstance.revision = newRevision;

      expect(documentInstance.revision).to.deep.equal(newRevision);
    });
  });

  describe('entropy', () => {
    it('should set entropy', () => {
      const documentInstance = createDocument({ id });

      const newEntropy = new Array(documentInstance.entropy.length).fill(0);

      documentInstance.entropy = newEntropy;

      expect(Array.from(documentInstance.entropy)).to.deep.equal(newEntropy);
    });
  });

  describe('createdAt', () => {
    it('should set created at', () => {
      const documentInstance = createDocument({ id });

      const createdAt = BigInt(new Date(1123).getTime());

      documentInstance.createdAt = createdAt;

      expect(documentInstance.createdAt).to.deep.equal(createdAt);
    });
  });

  describe('updatedAt', () => {
    it('should set updated at', () => {
      const documentInstance = createDocument({ id });

      const updatedAt = BigInt(new Date(1123).getTime());

      documentInstance.updatedAt = updatedAt;

      expect(documentInstance.updatedAt).to.deep.equal(updatedAt);
    });
  });

  describe('transferredAt', () => {
    it('should set transferred at', () => {
      const documentInstance = createDocument({ id });

      const transferredAt = BigInt(new Date(11231).getTime());

      documentInstance.transferredAt = transferredAt;

      expect(documentInstance.transferredAt).to.deep.equal(transferredAt);
    });
  });

  describe('createdAtBlockHeight', () => {
    it('should set created at Block Height', () => {
      const documentInstance = createDocument({ id });

      const createdAtHeight = BigInt(9172);

      documentInstance.createdAtBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtBlockHeight).to.deep.equal(createdAtHeight);
    });
  });

  describe('updatedAtBlockHeight', () => {
    it('should set updated at Block Height', () => {
      const documentInstance = createDocument({ id });

      const updatedAtHeight = BigInt(9172);

      documentInstance.updatedAtBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtBlockHeight).to.deep.equal(updatedAtHeight);
    });
  });

  describe('transferredAtBlockHeight', () => {
    it('should set transferred at Block Height', () => {
      const documentInstance = createDocument({ id });

      const transferredAtHeight = BigInt(9172);

      documentInstance.transferredAtBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtBlockHeight).to.deep.equal(transferredAtHeight);
    });
  });

  describe('createdAtCoreBlockHeight', () => {
    it('should set created at core Block Height', () => {
      const documentInstance = createDocument({ id });

      const createdAtHeight = 91721;

      documentInstance.createdAtCoreBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtCoreBlockHeight).to.deep.equal(createdAtHeight);
    });
  });

  describe('updatedAtCoreBlockHeight', () => {
    it('should set updated at core Block Height', () => {
      const documentInstance = createDocument({ id });

      const updatedAtHeight = 91722;

      documentInstance.updatedAtCoreBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtCoreBlockHeight).to.deep.equal(updatedAtHeight);
    });
  });

  describe('transferredAtCoreBlockHeight', () => {
    it('should set transferred at core Block Height', () => {
      const documentInstance = createDocument({ id });

      const transferredAtHeight = 91723;

      documentInstance.transferredAtCoreBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtCoreBlockHeight).to.deep.equal(transferredAtHeight);
    });
  });

  describe('documentTypeName', () => {
    it('should set document type name', () => {
      const documentInstance = createDocument({ id });

      const newDocumentTypeName = 'bbbb';

      documentInstance.documentTypeName = newDocumentTypeName;

      expect(documentInstance.documentTypeName).to.deep.equal(newDocumentTypeName);
    });
  });
});
