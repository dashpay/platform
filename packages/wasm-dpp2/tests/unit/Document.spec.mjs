import getWasm from './helpers/wasm.js';
import {
  document, dataContractId, ownerId, documentTypeName, revision, dataContractValue, id, document2, documentBytes,
} from './mocks/Document/index.js';
import { fromHexString } from './utils/hex.js';

let wasm;
let PlatformVersion;

before(async () => {
  wasm = await getWasm();
  ({ PlatformVersion } = wasm);
});

describe('Document', () => {
  describe('serialization / deserialization', () => {
    it('should allows to create Document from values', () => {
      const dataContractIdentifier = new wasm.Identifier(dataContractId);
      const ownerIdentifier = new wasm.Identifier(ownerId);

      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractIdentifier, ownerIdentifier);

      expect(documentInstance.__wbg_ptr).to.not.equal(0);
    });

    it('should allows to create Document from values with custom id', () => {
      const dataContractIdentifier = new wasm.Identifier(dataContractId);
      const ownerIdentifier = new wasm.Identifier(ownerId);
      const identifier = new wasm.Identifier(id);

      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractIdentifier, ownerIdentifier, identifier);

      expect(documentInstance.__wbg_ptr).to.not.equal(0);
    });

    it('should allows to create Document from bytes and convert to bytes', () => {
      const dataContract = wasm.DataContract.fromValue(dataContractValue, false);
      const documentInstance = wasm.Document.fromBytes(fromHexString(documentBytes), dataContract, 'note');

      const bytes = documentInstance.bytes(dataContract, PlatformVersion.PLATFORM_V1);

      expect(documentInstance.dataContractId.base58()).to.equal(dataContract.id.base58());
      expect(bytes).to.deep.equal(fromHexString(documentBytes));
      expect(dataContract.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should return document id', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      expect(documentInstance.id.base58()).to.deep.equal(id);
    });

    it('should return owner id', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      expect(documentInstance.ownerId.base58()).to.deep.equal(ownerId);
    });

    it('should return data contract id', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      expect(documentInstance.dataContractId.base58()).to.deep.equal(dataContractId);
    });

    it('should return properties', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      expect(documentInstance.properties).to.deep.equal(document);
    });

    it('should return revision', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      expect(documentInstance.revision).to.deep.equal(revision);
    });
  });

  describe('setters', () => {
    it('should allow to set document id', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      documentInstance.id = ownerId;

      expect(documentInstance.id.base58()).to.deep.equal(ownerId);
    });

    it('should allow to set document owner id', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      documentInstance.ownerId = id;

      expect(documentInstance.ownerId.base58()).to.deep.equal(id);
    });

    it('should allow to set entropy', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const newEntropy = new Array(documentInstance.entropy.length).fill(0);

      documentInstance.entropy = newEntropy;

      expect(Array.from(documentInstance.entropy)).to.deep.equal(newEntropy);
    });

    it('should allow to set properties', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      documentInstance.properties = document2;

      expect(documentInstance.properties).to.deep.equal(document2);
    });

    it('should allow to set revision', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const newRevision = BigInt(1000);

      documentInstance.revision = newRevision;

      expect(documentInstance.revision).to.deep.equal(newRevision);
    });

    it('should allow to set created at', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const createdAt = BigInt(new Date(1123).getTime());

      documentInstance.createdAt = createdAt;

      expect(documentInstance.createdAt).to.deep.equal(createdAt);
    });

    it('should allow to set updated at', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const updatedAt = BigInt(new Date(1123).getTime());

      documentInstance.updatedAt = updatedAt;

      expect(documentInstance.updatedAt).to.deep.equal(updatedAt);
    });

    it('should allow to set transferred at', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const transferredAt = BigInt(new Date(11231).getTime());

      documentInstance.transferredAt = transferredAt;

      expect(documentInstance.transferredAt).to.deep.equal(transferredAt);
    });

    it('should allow to set create at Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const createdAtHeight = BigInt(9172);

      documentInstance.createdAtBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtBlockHeight).to.deep.equal(createdAtHeight);
    });

    it('should allow to set updated at Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const updatedAtHeight = BigInt(9172);

      documentInstance.updatedAtBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtBlockHeight).to.deep.equal(updatedAtHeight);
    });

    it('should allow to set transferred at Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const transferredAtHeight = BigInt(9172);

      documentInstance.transferredAtBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtBlockHeight).to.deep.equal(transferredAtHeight);
    });

    it('should allow to set create at core Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const createdAtHeight = 91721;

      documentInstance.createdAtCoreBlockHeight = createdAtHeight;

      expect(documentInstance.createdAtCoreBlockHeight).to.deep.equal(createdAtHeight);
    });

    it('should allow to set updated at Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const updatedAtHeight = 91722;

      documentInstance.updatedAtCoreBlockHeight = updatedAtHeight;

      expect(documentInstance.updatedAtCoreBlockHeight).to.deep.equal(updatedAtHeight);
    });

    it('should allow to set transferred at Block Height', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

      const transferredAtHeight = 91723;

      documentInstance.transferredAtCoreBlockHeight = transferredAtHeight;

      expect(documentInstance.transferredAtCoreBlockHeight).to.deep.equal(transferredAtHeight);
    });

    it('should allow to set document type name', () => {
      const documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);

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
