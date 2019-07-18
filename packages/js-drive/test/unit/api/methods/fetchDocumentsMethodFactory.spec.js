const fetchDocumentsMethodFactory = require('../../../../lib/api/methods/fetchDocumentsMethodFactory');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const ValidationError = require('../../../../lib/stateView/document/query/errors/ValidationError');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const InvalidQueryError = require('../../../../lib/stateView/document/errors/InvalidQueryError');

describe('fetchDocumentsMethodFactory', () => {
  let contractId;
  let type;
  let options;
  let fetchDocumentsMock;
  let fetchDocumentsMethod;

  beforeEach(function beforeEach() {
    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    type = 'niceDocument';
    options = {};

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMethod = fetchDocumentsMethodFactory(fetchDocumentsMock);
  });

  it('should throw InvalidParamsError if "type" is not provided', async () => {
    try {
      await fetchDocumentsMethod({ contractId });

      expect.fail('should throw InvalidParamsError error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidParamsError);
      expect(e).to.have.property('message', 'Missing "type" param');

      expect(fetchDocumentsMock).to.have.not.been.called();
    }
  });

  it('should throw InvalidParamsError if "contractId" is not provided', async () => {
    try {
      await fetchDocumentsMethod({ type });

      expect.fail('should throw InvalidParamsError error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidParamsError);
      expect(e).to.have.property('message', 'Missing "contractId" param');

      expect(fetchDocumentsMock).to.have.not.been.called();
    }
  });

  it('should throw InvalidParamsError if InvalidWhereError is thrown', async () => {
    const validationErrors = [new ValidationError('something')];
    const error = new InvalidQueryError(validationErrors);

    fetchDocumentsMock.throws(error);

    try {
      await fetchDocumentsMethod({ contractId, type, options });

      expect.fail('should throw InvalidParamsError error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidParamsError);
      expect(e).to.have.property('message', error.message);
      expect(e).to.have.property('data', error.getErrors());

      expect(fetchDocumentsMock).to.have.been.calledOnceWith(contractId, type, options);
    }
  });

  it('should escalate an error if error type is unknown', async () => {
    const fetchError = new Error();

    fetchDocumentsMock.throws(fetchError);

    let error;
    try {
      await fetchDocumentsMethod({ contractId, type, options });
    } catch (e) {
      error = e;
    }

    expect(error).to.equal(fetchError);

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(contractId, type, options);
  });

  it('should return Documents', async () => {
    const documents = getDocumentsFixture();
    const rawDocuments = documents.map(d => d.toJSON());

    fetchDocumentsMock.resolves(documents);

    const result = await fetchDocumentsMethod({ contractId, type, options });

    expect(result).to.deep.equal(rawDocuments);

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(contractId, type, options);
  });
});
