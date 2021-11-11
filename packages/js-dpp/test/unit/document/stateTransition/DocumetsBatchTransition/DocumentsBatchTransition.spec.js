const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const stateTransitionTypes = require('../../../../../lib/stateTransition/stateTransitionTypes');
const createDPPMock = require('../../../../../lib/test/mocks/createDPPMock');
const protocolVersion = require('../../../../../lib/version/protocolVersion');
const DocumentFactory = require('../../../../../lib/document/DocumentFactory');
const serializer = require('../../../../../lib/util/serializer');
const hash = require('../../../../../lib/util/hash');

describe('DocumentsBatchTransition', () => {
  let stateTransition;
  let documents;
  let hashMock;
  let encodeMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);

    encodeMock = this.sinonSandbox.stub(serializer, 'encode');
    hashMock = this.sinonSandbox.stub(hash, 'hash');

    const factory = new DocumentFactory(createDPPMock(), undefined, undefined);
    stateTransition = factory.createStateTransition({
      create: documents,
    });
  });

  afterEach(() => {
    encodeMock.restore();
    hashMock.restore();
  });

  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(protocolVersion.latestVersion);
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(stateTransitionTypes.DOCUMENTS_BATCH);
    });
  });

  describe('#getTransitions', () => {
    it('should return document transitions', () => {
      const result = stateTransition.getTransitions();

      expect(result).to.equal(stateTransition.transitions);
    });
  });

  describe('#toJSON', () => {
    it('should return State Transition as JSON', () => {
      expect(stateTransition.toJSON()).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documents[0].getOwnerId().toString(),
        transitions: stateTransition.getTransitions().map((d) => d.toJSON()),
        signaturePublicKeyId: undefined,
        signature: undefined,
      });
    });
  });

  describe('#toObject', () => {
    it('should return State Transition as plain object', () => {
      expect(stateTransition.toObject()).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documents[0].getOwnerId(),
        transitions: stateTransition.getTransitions().map((d) => d.toObject()),
        signaturePublicKeyId: undefined,
        signature: undefined,
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized Documents State Transition', () => {
      const serializedStateTransition = Buffer.from('123');

      encodeMock.returns(serializedStateTransition);

      const result = stateTransition.toBuffer();

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(stateTransition.protocolVersion, 0);

      expect(result).to.deep.equal(
        Buffer.concat([protocolVersionUInt32, serializedStateTransition]),
      );

      const dataToEncode = stateTransition.toObject();
      delete dataToEncode.protocolVersion;

      expect(encodeMock).to.have.been.calledOnceWith(dataToEncode);
    });
  });

  describe('#hash', () => {
    it('should return Documents State Transition hash as hex', () => {
      const serializedDocument = Buffer.from('123');
      const hashedDocument = '456';

      encodeMock.returns(serializedDocument);
      hashMock.returns(hashedDocument);

      const result = stateTransition.hash();

      expect(result).to.equal(hashedDocument);

      const dataToEncode = stateTransition.toObject();
      delete dataToEncode.protocolVersion;

      expect(encodeMock).to.have.been.calledOnceWith(dataToEncode);

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(stateTransition.protocolVersion, 0);

      expect(hashMock).to.have.been.calledOnceWith(
        Buffer.concat([protocolVersionUInt32, serializedDocument]),
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();

      expect(result).to.deep.equal(getDocumentsFixture.ownerId);
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of affected documents', () => {
      const expectedIds = documents.map((doc) => doc.getId());
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(10);
      expect(result).to.be.deep.equal(expectedIds);
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.true();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.false();
    });
  });
});
