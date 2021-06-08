const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const populateMongoDbTransactionFromObjectFactory = require('../../../lib/document/populateMongoDbTransactionFromObjectFactory');
const DocumentsDBTransactionIsNotStartedError = require('../../../lib/document/errors/DocumentsDBTransactionIsNotStartedError');

describe('populateMongoDbTransactionFromObjectFactory', () => {
  let populateMongoDbTransactionFromObject;
  let createPreviousDocumentMongoDbRepositoryMock;
  let dppMock;
  let mongoDbRepositoryMock;
  let transactionMock;
  let transactionObjectMock;
  let dataToUpdate;
  let dataToDelete;
  let documentToCreate;
  let documentToDelete;

  beforeEach(function beforeEach() {
    [documentToCreate, documentToDelete] = getDocumentsFixture();

    dataToUpdate = {
      documentIdToCreate: documentToCreate.toBuffer(),
    };

    dataToDelete = {
      documentIdToDelete: documentToDelete.toBuffer(),
    };

    transactionMock = {
      isStarted: this.sinon.stub(),
    };

    transactionObjectMock = {
      updates: dataToUpdate,
      deletes: dataToDelete,
    };

    mongoDbRepositoryMock = {
      delete: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    createPreviousDocumentMongoDbRepositoryMock = this.sinon.stub().resolves(mongoDbRepositoryMock);

    dppMock = createDPPMock(this.sinon);
    dppMock.document.createFromBuffer.resolves(documentToCreate);

    populateMongoDbTransactionFromObject = populateMongoDbTransactionFromObjectFactory(
      createPreviousDocumentMongoDbRepositoryMock,
      dppMock,
    );
  });

  it('should throw DocumentsDBTransactionIsNotStartedError error if transaction is not started', async () => {
    transactionMock.isStarted.returns(false);

    try {
      await populateMongoDbTransactionFromObject(transactionMock, transactionObjectMock);

      expect.fail('should throw DocumentsDBTransactionIsNotStartedError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(DocumentsDBTransactionIsNotStartedError);
    }
  });

  it('should return updated transaction', async () => {
    transactionMock.isStarted.returns(true);

    dppMock.document.createFromBuffer.onCall(1).resolves(documentToDelete);

    await populateMongoDbTransactionFromObject(transactionMock, transactionObjectMock);

    expect(dppMock.document.createFromBuffer).to.be.calledTwice();
    expect(dppMock.document.createFromBuffer.getCall(0)).to.be.calledWithExactly(
      documentToCreate.toBuffer(),
      {
        skipValidation: true,
      },
    );
    expect(dppMock.document.createFromBuffer.getCall(1)).to.be.calledWithExactly(
      documentToDelete.toBuffer(),
      {
        skipValidation: true,
      },
    );

    expect(createPreviousDocumentMongoDbRepositoryMock).to.be.calledTwice();
    expect(createPreviousDocumentMongoDbRepositoryMock.getCall(0)).to.be.calledWithExactly(
      documentToCreate.getDataContractId(),
      documentToCreate.getType(),
    );
    expect(createPreviousDocumentMongoDbRepositoryMock.getCall(1)).to.be.calledWithExactly(
      documentToDelete.getDataContractId(),
      documentToDelete.getType(),
    );

    expect(mongoDbRepositoryMock.store).to.be.calledOnceWithExactly(
      documentToCreate,
      transactionMock,
    );
    expect(mongoDbRepositoryMock.delete).to.be.calledOnceWithExactly(
      documentToDelete.getId(),
      transactionMock,
    );
  });
});
