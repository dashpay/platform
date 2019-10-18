const rewiremock = require('rewiremock/node');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

describe('DocumentsStateTransition', () => {
  let stateTransition;
  let documents;
  let hashMock;
  let encodeMock;
  let DocumentsStateTransition;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DocumentsStateTransition = rewiremock.proxy('../../../../lib/document/stateTransition/DocumentsStateTransition', {
      '../../../../lib/util/hash': hashMock,
      '../../../../lib/util/serializer': serializerMock,
    });

    documents = getDocumentsFixture();
    stateTransition = new DocumentsStateTransition(documents);
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

      expect(result).to.equal(stateTransitionTypes.DOCUMENTS);
    });
  });

  describe('#getDocuments', () => {
    it('should return documents', () => {
      const result = stateTransition.getDocuments();

      expect(result).to.equal(documents);
    });
  });

  describe('#setDocuments', () => {
    it('should set documents and actions', () => {
      const result = stateTransition.setDocuments([]);

      expect(result).to.be.an.instanceOf(DocumentsStateTransition);
      expect(stateTransition.documents).to.deep.equal([]);

      stateTransition.setDocuments(documents);

      expect(stateTransition.documents).to.equal(documents);
    });
  });

  describe('#toJSON', () => {
    it('should return State Transition as plain JS object', () => {
      expect(stateTransition.toJSON()).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.DOCUMENTS,
        actions: documents.map(d => d.getAction()),
        documents: documents.map(d => d.toJSON()),
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized Documents State Transition', () => {
      const serializedStateTransition = '123';

      encodeMock.returns(serializedStateTransition);

      const result = stateTransition.serialize();

      expect(result).to.equal(serializedStateTransition);

      expect(encodeMock).to.have.been.calledOnceWith(stateTransition.toJSON());
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

      expect(encodeMock).to.have.been.calledOnceWith(stateTransition.toJSON());
      expect(hashMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });
});
