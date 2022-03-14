const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const DocumentRepository = require('../../../lib/document/DocumentRepository');
const GroveDBStoreMock = require('../../../lib/test/mock/GroveDBStoreMock');
const LoggerMock = require('../../../lib/test/mock/LoggerMock');
const DriveMock = require('../../../lib/test/mock/DriveMock');
const createDocumentTypeTreePath = require('../../../lib/document/groveDB/createDocumentTreePath');
const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');
const ValidationError = require('../../../lib/document/query/errors/ValidationError');

describe('DocumentRepository', () => {
  let document;
  let repository;
  let dppMock;
  let groveDBStoreMock;
  let loggerMock;
  let appHash;
  let driveMock;
  let dataContract;
  let validateQueryMock;
  let validateQueryResult;

  beforeEach(function beforeEach() {
    appHash = Buffer.alloc(0);

    [document] = getDocumentsFixture();
    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .document
      .createFromBuffer
      .resolves(document);

    validateQueryResult = {
      isValid: this.sinon.stub().returns(true),
      getErrors: this.sinon.stub(),
    };

    validateQueryMock = this.sinon.stub().returns(validateQueryResult);

    driveMock = new DriveMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.logger = loggerMock;
    groveDBStoreMock.getRootHash.resolves(appHash);
    groveDBStoreMock.getDrive.returns(driveMock);

    repository = new DocumentRepository(groveDBStoreMock, validateQueryMock, loggerMock);
  });

  describe('#store', () => {
    it('should store document', async () => {
      await repository.store(document, false);

      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      expect(groveDBStoreMock.get).to.be.calledOnceWithExactly(
        documentTreePath,
        document.getId().toBuffer(),
        { useTransaction: false },
      );

      expect(driveMock.createDocument).to.be.calledOnceWithExactly(
        document,
        false,
      );

      expect(loggerMock.info).to.be.calledOnce();
    });

    it('should update document', async () => {
      groveDBStoreMock.get.resolves(document);

      await repository.store(document, true);

      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      expect(groveDBStoreMock.get).to.be.calledOnceWithExactly(
        documentTreePath,
        document.getId().toBuffer(),
        { useTransaction: true },
      );

      expect(driveMock.updateDocument).to.be.calledOnceWithExactly(
        document,
        true,
      );
    });
  });

  describe('#isExist', () => {
    it('should return true is document exists', async () => {
      groveDBStoreMock.get.resolves(document);

      const result = await repository.isExist(document, true);

      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      expect(groveDBStoreMock.get).to.be.calledOnceWithExactly(
        documentTreePath,
        document.getId().toBuffer(),
        { useTransaction: true },
      );

      expect(result).to.be.true();
    });

    it('should return false is document does not exist', async () => {
      groveDBStoreMock.get.resolves(null);

      const result = await repository.isExist(document, false);

      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      expect(groveDBStoreMock.get).to.be.calledOnceWithExactly(
        documentTreePath,
        document.getId().toBuffer(),
        { useTransaction: false },
      );

      expect(result).to.be.false();
    });
  });

  describe('#find', () => {
    it('should throw InvalidQueryError is query is invalid', async () => {
      const error = new ValidationError('Validation error');

      validateQueryResult.isValid.returns(false);
      validateQueryResult.getErrors.returns([error]);

      try {
        await repository.find(dataContract, document.getType(), {}, false);

        expect.fail('should throw InvalidQueryError');
      } catch (e) {
        expect(e).to.be.an.instanceof(InvalidQueryError);
        expect(e.getErrors()).to.deep.equal([error]);
      }
    });

    it('should return documents', async () => {
      groveDBStoreMock.get.returns(null);
      driveMock.queryDocuments.resolves([document]);

      const query = {
        optionA: '1',
        optionB: undefined,
      };

      const result = await repository.find(dataContract, document.getType(), query, true);

      expect(result).to.deep.equal([document]);

      expect(driveMock.queryDocuments).to.be.calledOnceWithExactly(
        dataContract,
        document.getType(),
        {
          optionA: '1',
        },
        true,
      );
    });
  });

  describe('#delete', () => {
    it('should delete document', async () => {
      await repository.delete(dataContract, document.getType(), document.getId(), true);

      expect(driveMock.deleteDocument).to.be.calledOnceWithExactly(
        dataContract,
        document.getType(),
        document.getId(),
        true,
      );

      expect(loggerMock.info).to.be.calledOnce();
    });
  });
});
