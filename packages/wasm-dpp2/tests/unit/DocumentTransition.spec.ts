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

let documentInstance: InstanceType<typeof wasm.Document>;
let createTransition: InstanceType<typeof wasm.DocumentCreateTransition>;
let replaceTransition: InstanceType<typeof wasm.DocumentReplaceTransition>;

describe('DocumentTransition', () => {
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

  before(async () => {
    documentInstance = createDocument();
    createTransition = new wasm.DocumentCreateTransition({
      document: documentInstance, identityContractNonce: BigInt(1),
    });
    replaceTransition = new wasm.DocumentReplaceTransition({
      document: documentInstance, identityContractNonce: BigInt(1),
    });
  });

  describe('toDocumentTransition()', () => {
    it('should create DocumentTransition from create transition', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition).to.be.an.instanceof(wasm.DocumentTransition);
    });
  });

  describe('actionType', () => {
    it('should return action type', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.actionType).to.equal('create');
    });
  });

  describe('dataContractId', () => {
    it('should return dataContractId', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.dataContractId.toBase58()).to.deep.equal(documentInstance.dataContractId.toBase58());
    });

    it('should set dataContractId', () => {
      const documentTransition = createTransition.toDocumentTransition();

      documentTransition.dataContractId = new Uint8Array(32);

      expect(documentTransition.dataContractId.toBytes()).to.deep.equal(new Uint8Array(32));
    });
  });

  describe('id', () => {
    it('should return id', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.id.toBase58()).to.deep.equal(documentInstance.id.toBase58());
    });
  });

  describe('documentTypeName', () => {
    it('should return documentTypeName', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.documentTypeName).to.equal(documentTypeName);
    });
  });

  describe('identityContractNonce', () => {
    it('should return identityContractNonce', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.identityContractNonce).to.equal(BigInt(1));
    });

    it('should set identityContractNonce', () => {
      const documentTransition = createTransition.toDocumentTransition();

      documentTransition.identityContractNonce = BigInt(3333);

      expect(documentTransition.identityContractNonce).to.equal(BigInt(3333));
    });
  });

  describe('revision', () => {
    it('should return revision', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.revision).to.equal(BigInt(1));
    });

    it('should set revision', () => {
      const documentTransition = replaceTransition.toDocumentTransition();

      documentTransition.revision = BigInt(123);

      expect(documentTransition.revision).to.equal(BigInt(123));
    });
  });

  describe('entropy', () => {
    it('should return entropy', () => {
      const documentTransition = createTransition.toDocumentTransition();

      expect(documentTransition.entropy).to.deep.equal(documentInstance.entropy);
    });
  });
});
