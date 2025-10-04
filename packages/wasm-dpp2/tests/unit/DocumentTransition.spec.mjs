import getWasm from './helpers/wasm.js';
import {
  document, documentTypeName, revision, dataContractId, ownerId, id,
} from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

let documentInstance;
let createTransition;
let replaceTransition;

describe('DocumentTransition', () => {
  before(async () => {
    documentInstance = new wasm.Document(document, documentTypeName, revision, dataContractId, ownerId, id);
    createTransition = new wasm.DocumentCreateTransition(documentInstance, BigInt(1));
    replaceTransition = new wasm.DocumentReplaceTransition(documentInstance, BigInt(1));
  });

  describe('serialization / deserialization', () => {
    it('should allow to create from documents document_transitions', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get action type', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.actionType).to.equal('create');
    });

    it('should allow to get dataContractId', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.dataContractId.base58()).to.deep.equal(documentInstance.dataContractId.base58());
    });

    it('should allow to get id', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.id.base58()).to.deep.equal(documentInstance.id.base58());
    });

    it('should allow to get documentTypeName', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.documentTypeName).to.equal(documentTypeName);
    });

    it('should allow to get identityContractNonce', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.identityContractNonce).to.equal(BigInt(1));
    });

    it('should allow to get revision', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.revision).to.equal(BigInt(1));
    });

    it('should allow to get entropy', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.entropy).to.deep.equal(documentInstance.entropy);
    });
  });

  describe('setters', () => {
    it('should allow to set dataContractId', () => {
      const documentTransition = createTransition.toDocumentTransition();

      documentTransition.dataContractId = new Uint8Array(32);

      expect(documentTransition.dataContractId.bytes()).to.deep.equal(new Uint8Array(32));
    });

    it('should allow to set identityContractNonce', () => {
      const documentTransition = createTransition.toDocumentTransition();

      documentTransition.identityContractNonce = BigInt(3333);

      expect(documentTransition.identityContractNonce).to.equal(BigInt(3333));
    });

    it('should allow to set revision', () => {
      const documentTransition = replaceTransition.toDocumentTransition();

      documentTransition.revision = BigInt(123);

      expect(documentTransition.revision).to.equal(BigInt(123));
    });
  });
});
