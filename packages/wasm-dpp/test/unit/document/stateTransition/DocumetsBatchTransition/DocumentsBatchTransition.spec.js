const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const DocumentFactoryJs = require('@dashevo/dpp/lib/document/DocumentFactory');
const lodash = require('lodash');
const { default: loadWasmDpp } = require('../../../../../dist');

let Identifier;
let DocumentFactory;
let DataContract;
let Document;
let DocumentValidator;
let ProtocolVersionValidator;
let DocumentsContainer;

function newDocumentsContainer(documents) {
  const {
    // use constants
    ["create"]: createDocuments,
    ["replace"]: replaceDocuments,
    ["delete"]: deleteDocuments,
  } = documents;
  const documentsContainer = new DocumentsContainer;
  if (createDocuments != null) {
    for (d of createDocuments) {
      documentsContainer.pushDocumentCreate(d);
    }
  }
  if (replaceDocuments != null) {
    for (d of replaceDocuments) {
      documentsContainer.pushDocumentReplace(d);
    }
  }
  if (deleteDocuments != null) {
    for (d of deleteDocuments) {
      documentsContainer.pushDeleteDocument(d);
    }
  }

  return documentsContainer;
}


describe('DocumentsBatchTransition', () => {
  let stateTransitionJs;
  let stateTransition;
  let documentsJs;
  let documents;
  let hashMock;
  let encodeMock;
  let dataContractJs;
  let dataContract;
  let factoryJs;

  beforeEach(async () => {
    ({
      Identifier, ProtocolVersionValidator, DocumentValidator, DocumentFactory, DataContract, Document, DocumentTransitions, Documents, DocumentsContainer
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {

    dataContractJs = getDataContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      let doc = new Document(d.toObject(), dataContract)
      doc.setEntropy(d.entropy);
      return doc;

    });

    const protocolVersionValidatorRs = new ProtocolVersionValidator();
    const documentValidatorRs = new DocumentValidator(protocolVersionValidatorRs);
    const factory = new DocumentFactory(1, documentValidatorRs, {});
    factoryJs = new DocumentFactoryJs(createDPPMock(), undefined, undefined);

    stateTransitionJs = factoryJs.createStateTransition({
      create: documentsJs,
    });

    stateTransition = factory.createStateTransition(newDocumentsContainer({
      create: documents,
    }));
  });


  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransitionJs.getProtocolVersion();

      expect(result).to.equal(protocolVersion.latestVersion);
    });
  });

  describe('#getProtocolVersion - Rust', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(protocolVersion.latestVersion);
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransitionJs.getType();

      expect(result).to.equal(stateTransitionTypes.DOCUMENTS_BATCH);
    });
  });

  describe('#getType - Rust', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(stateTransitionTypes.DOCUMENTS_BATCH);
    });
  });

  describe('#getTransitions', () => {
    it('should return document transitions', () => {
      const result = stateTransitionJs.getTransitions();

      expect(result).to.equal(stateTransitionJs.transitions);
    });
  });

  describe('#getTransitions - Rust', () => {
    it('should return document transitions', () => {
      const transitionsJs = stateTransitionJs.getTransitions().map((t) => { return t.toJSON() });
      const transitions = stateTransition.getTransitions().map((t) => { return t.toJSON() });
      expect(transitionsJs).to.deep.equal(transitions)
    });
  });

  describe('#toJSON', () => {
    it('should return State Transition as JSON', () => {
      expect(stateTransitionJs.toJSON()).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documentsJs[0].getOwnerId().toString(),
        transitions: stateTransitionJs.getTransitions().map((d) => d.toJSON()),
        signaturePublicKeyId: undefined,
        signature: undefined,
      });
    });

    it('should return State Transition as JSON - Rust', () => {
      expect(stateTransition.toJSON()).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documentsJs[0].getOwnerId().toString(),
        transitions: stateTransitionJs.getTransitions().map((d) => { return d.toJSON() }),
        signaturePublicKeyId: undefined,
        signature: undefined,
      });
    });

  });

  describe('#toObject', () => {
    it('should return State Transition as plain object', () => {
      expect(stateTransitionJs.toObject()).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documentsJs[0].getOwnerId(),
        transitions: stateTransitionJs.getTransitions().map((d) => d.toObject()),
        signaturePublicKeyId: undefined,
        signature: undefined,
      });
    });

    it('should return State Transition as plain object -  Rust', () => {
      const rawObject = stateTransition.toObject();
      const rawObjectJs = stateTransitionJs.toObject();
      const rawObjectWithBuffers = lodash.cloneDeepWith(rawObject, (value) => {
        if (value instanceof Identifier) {
          return value.toBuffer();
        }
      });

      expect(rawObjectWithBuffers).to.deep.equal(rawObjectJs);
    });
  });

  describe('#toBuffer', () => {
    it('should return the same bytes as JS version', () => {
      const bufferJs = stateTransitionJs.toBuffer();
      const buffer = stateTransition.toBuffer();

      expect(bufferJs).to.deep.equal(buffer);
    });

  });



  describe('#hash', () => {
    it('should return the same hash as the JS version', () => {
      const hashJs = stateTransitionJs.hash();
      const hash = stateTransitionJs.hash();

      expect(hash).to.deep.equal(hashJs);
    });

  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransitionJs.getOwnerId();

      expect(result).to.deep.equal(getDocumentsFixture.ownerId);
    });
  });

  describe('#getOwnerId - Rust', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();

      expect(result.toBuffer()).to.deep.equal(getDocumentsFixture.ownerId.toBuffer());
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of affected documents', () => {
      const expectedIds = documentsJs.map((doc) => doc.getId());
      const result = stateTransitionJs.getModifiedDataIds();

      expect(result.length).to.be.equal(10);
      expect(result).to.be.deep.equal(expectedIds);
    });
  });

  describe('#getModifiedDataIds - Rust', () => {
    it('should return ids of affected documents', () => {
      const expectedIds = documentsJs.map((doc) => doc.getId().toBuffer());
      const result = stateTransition.getModifiedDataIds().map((id) => { return id.toBuffer() });

      expect(result.length).to.be.equal(10);
      expect(result).to.be.deep.equal(expectedIds);
    });
  });


  describe('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransitionJs.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDataContractStateTransition - Rust', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return true', () => {
      expect(stateTransitionJs.isDocumentStateTransition()).to.be.true();
    });
  });

  describe('#isDocumentStateTransition - Rust', () => {
    it('should return true', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.true();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return false', () => {
      expect(stateTransitionJs.isIdentityStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition - Rust', () => {
    it('should return false', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.false();
    });
  });
});
