const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const StoreMock = require('../../../lib/test/mock/StoreMock');

const DocumentStoreRepository = require('../../../lib/document/DocumentStoreRepository');

describe('DocumentStoreRepository', () => {
  let document;
  let repository;
  let dppMock;
  let storeMock;
  let transactionMock;

  beforeEach(function beforeEach() {
    [document] = getDocumentsFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .document
      .createFromBuffer
      .resolves(document);

    const containerMock = {
      resolve() {
        return dppMock;
      },
    };

    storeMock = new StoreMock(this.sinon);

    transactionMock = {};

    repository = new DocumentStoreRepository(storeMock, containerMock);
  });

  describe('#store', () => {
    it('should store document', async () => {
      await repository.store(document, transactionMock);

      expect(storeMock.put).to.be.calledOnceWithExactly(
        document.getId(),
        document.toBuffer(),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return null if document is not present', async () => {
      storeMock.get.returns(null);

      const result = await repository.fetch(document.getId(), transactionMock);

      expect(result).to.be.null();

      expect(storeMock.get).to.be.calledOnceWithExactly(
        document.getId(),
        transactionMock,
      );
    });

    it('should return document', async () => {
      const encodedDocument = document.toBuffer();

      storeMock.get.returns(encodedDocument);

      const result = await repository.fetch(document.getId(), transactionMock);

      expect(result).to.be.deep.equal(document);

      expect(storeMock.get).to.be.calledOnceWithExactly(
        document.getId(),
        transactionMock,
      );
    });
  });

  describe('#delete', () => {
    it('should delete document', async () => {
      await repository.delete(document.getId(), transactionMock);

      expect(storeMock.delete).to.be.calledOnceWithExactly(
        document.getId(),
        transactionMock,
      );
    });
  });
});
