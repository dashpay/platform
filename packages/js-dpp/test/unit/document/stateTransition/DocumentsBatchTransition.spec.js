const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const DocumentsBatchTransition = require(
  '../../../../lib/document/stateTransition/DocumentsBatchTransition',
);

describe('DocumentsBatchTransition', () => {
  let stateTransition;
  let documents;
  let hashMock;
  let encodeMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);

    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    const DocumentFactory = rewiremock.proxy('../../../../lib/document/DocumentFactory', {
      '../../../../lib/util/hash': hashMock,
      '../../../../lib/util/serializer': serializerMock,
    });

    const factory = new DocumentFactory(undefined, undefined);
    stateTransition = factory.createStateTransition({
      create: documents,
    });
  });

  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(0);
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
        protocolVersion: 0,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documents[0].getOwnerId(),
        transitions: stateTransition.getTransitions().map((d) => d.toJSON()),
        signaturePublicKeyId: null,
        signature: null,
      });
    });
  });

  describe('#toObject', () => {
    it('should return State Transition as plain object', () => {
      expect(stateTransition.toObject()).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.DOCUMENTS_BATCH,
        ownerId: documents[0].getOwnerId(),
        transitions: stateTransition.getTransitions().map((d) => d.toObject()),
        signaturePublicKeyId: null,
        signature: null,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized Documents State Transition', () => {
      const serializedStateTransition = '123';

      encodeMock.returns(serializedStateTransition);

      const result = stateTransition.serialize();

      expect(result).to.equal(serializedStateTransition);

      expect(encodeMock).to.have.been.calledOnceWith(stateTransition.toObject());
    });
  });

  describe('#hash', () => {
    it('should return Documents State Transition hash as hex', () => {
      const serializedDocument = '123';
      const hashedDocument = '456';

      encodeMock.returns(serializedDocument);
      hashMock.returns(hashedDocument);

      const result = stateTransition.hash();

      expect(result).to.equal(hashedDocument);

      expect(encodeMock).to.have.been.calledOnceWith(stateTransition.toObject());
      expect(hashMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();

      expect(result).to.equal(getDocumentsFixture.ownerId);
    });
  });

  describe('#fromJSON', () => {
    it('should create an instance using plain object converted from JSON', async () => {
      const rawStateTransition = stateTransition.toJSON();

      const result = DocumentsBatchTransition.fromJSON(
        rawStateTransition, [dataContract],
      );

      expect(result.toJSON()).to.deep.equal(
        stateTransition.toJSON(),
      );
    });
  });
});
