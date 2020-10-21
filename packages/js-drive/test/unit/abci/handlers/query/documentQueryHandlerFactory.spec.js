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
  let fetchDocumentsMock;
  let documents;
  let params;
  let data;
  let options;

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();

    fetchDocumentsMock = this.sinon.stub();

    documentQueryHandler = documentQueryHandlerFactory(
      fetchDocumentsMock,
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
    fetchDocumentsMock.resolves(documents);

    const result = await documentQueryHandler(params, data);

    expect(fetchDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const documentsResponse = cbor.encode(
      documents.map((document) => document.toBuffer()),
    );

    expect(result.value).to.deep.equal(documentsResponse);
  });

  it('should throw InvalidArgumentAbciError on invalid query', async () => {
    const error = new ValidationError('Some error');
    const queryError = new InvalidQueryError([error]);

    fetchDocumentsMock.throws(queryError);

    try {
      await documentQueryHandler(params, data);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({ errors: [error] });
      expect(fetchDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    }
  });

  it('should throw error if fetchDocuments throws unknown error', async () => {
    const error = new Error('Some error');

    fetchDocumentsMock.throws(error);

    try {
      await documentQueryHandler(params, data);

      expect.fail('should throw any error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(fetchDocumentsMock).to.be.calledOnceWith(data.contractId, data.type, options);
    }
  });
});
