const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const cbor = require('cbor');

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

  beforeEach(function beforeEach() {
    documents = getDocumentsFixture();

    fetchDocumentsMock = this.sinon.stub();

    documentQueryHandler = documentQueryHandlerFactory(
      fetchDocumentsMock,
    );
  });

  it('should return serialized documents', async () => {
    const contractId = 'contractId';
    const type = 'documentType';
    const options = {};

    fetchDocumentsMock.resolves(documents);

    const result = await documentQueryHandler({ contractId, type }, options);

    expect(fetchDocumentsMock).to.be.calledOnceWith(contractId, type, options);
    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    const documentsResponse = cbor.encode(
      documents.map((d) => d.serialize()),
    );

    expect(result.value).to.deep.equal(documentsResponse);
  });

  it('should throw InvalidArgumentAbciError on invalid query', async () => {
    const contractId = 'contractId';
    const type = 'documentType';
    const options = {};
    const error = new ValidationError('Some error');
    const queryError = new InvalidQueryError([error]);

    fetchDocumentsMock.throws(queryError);

    try {
      await documentQueryHandler({ contractId, type }, options);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({ errors: [error] });
      expect(fetchDocumentsMock).to.be.calledOnceWith(contractId, type, options);
    }
  });

  it('should throw error if fetchDocuments throws unknown error', async () => {
    const error = new Error('Some error');
    const contractId = 'contractId';
    const type = 'documentType';
    const options = {};

    fetchDocumentsMock.throws(error);

    try {
      await documentQueryHandler({ contractId, type }, options);

      expect.fail('should throw any error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(fetchDocumentsMock).to.be.calledOnceWith(contractId, type, options);
    }
  });
});
