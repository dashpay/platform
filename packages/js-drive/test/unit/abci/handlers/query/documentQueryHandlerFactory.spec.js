const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const cbor = require('cbor');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const documentQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/documentQueryHandlerFactory');
const InvalidQueryError = require('../../../../../lib/document/errors/InvalidQueryError');
const ValidationError = require('../../../../../lib/document/query/errors/ValidationError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('documentQueryHandlerFactory', () => {
  let documentQueryHandler;
  let fetchPreviousDocumentsMock;
  let documents;
  let params;
  let data;
  let options;
  let previousRootTreeMock;
  let previousDocumentsStoreRootTreeLeafMock;

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();

    fetchPreviousDocumentsMock = this.sinon.stub();

    previousRootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    previousDocumentsStoreRootTreeLeafMock = this.sinon.stub();

    documentQueryHandler = documentQueryHandlerFactory(
      fetchPreviousDocumentsMock,
      previousRootTreeMock,
      previousDocumentsStoreRootTreeLeafMock,
    );

    params = {};
    data = {
      contractId: generateRandomIdentifier(),
      type: 'documentType',
      orderBy: [{ sort: 'asc' }],
      limit: 2,
      startAt: 0,
      startAfter: undefined,
      where: [['field', '==', 'value']],
    };
    options = {
      orderBy: data.orderBy,
      limit: data.limit,
      startAt: data.startAt,
      startAfter: data.startAfter,
      where: data.where,
    };
  });

  it('should return serialized documents', async () => {
    fetchPreviousDocumentsMock.resolves(documents);

    const result = await documentQueryHandler(params, data, {});

    expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const value = {
      data: documents.map((document) => document.toBuffer()),
    };

    expect(result.value).to.deep.equal(cbor.encode(value));
    expect(previousRootTreeMock.getFullProof).to.be.not.called();
  });

  it('should return serialized documents with proof', async () => {
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    fetchPreviousDocumentsMock.resolves(documents);
    previousRootTreeMock.getFullProof.returns(proof);

    const result = await documentQueryHandler(params, data, { prove: 'true' });

    expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const value = {
      data: documents.map((document) => document.toBuffer()),
      proof,
    };

    const documentIds = documents.map((document) => document.getId());

    expect(result.value).to.deep.equal(cbor.encode(value));
    expect(previousRootTreeMock.getFullProof).to.be.calledOnce();
    expect(previousRootTreeMock.getFullProof.getCall(0).args).to.deep.equal([
      previousDocumentsStoreRootTreeLeafMock,
      documentIds,
    ]);
  });

  it('should throw InvalidArgumentAbciError on invalid query', async () => {
    const error = new ValidationError('Some error');
    const queryError = new InvalidQueryError([error]);

    fetchPreviousDocumentsMock.throws(queryError);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({ errors: [error] });
      expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    }
  });

  it('should throw error if fetchDocuments throws unknown error', async () => {
    const error = new Error('Some error');

    fetchPreviousDocumentsMock.throws(error);

    try {
      await documentQueryHandler(params, data, {});

      expect.fail('should throw any error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(fetchPreviousDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    }
  });
});
