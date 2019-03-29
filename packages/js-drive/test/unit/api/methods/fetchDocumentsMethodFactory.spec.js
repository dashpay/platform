const fetchDocumentsMethodFactory = require('../../../../lib/api/methods/fetchDocumentsMethodFactory');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');
const InvalidWhereError = require('../../../../lib/stateView/document/errors/InvalidWhereError');
const InvalidOrderByError = require('../../../../lib/stateView/document/errors/InvalidOrderByError');
const InvalidLimitError = require('../../../../lib/stateView/document/errors/InvalidLimitError');
const InvalidStartAtError = require('../../../../lib/stateView/document/errors/InvalidStartAtError');
const InvalidStartAfterError = require('../../../../lib/stateView/document/errors/InvalidStartAfterError');
const AmbiguousStartError = require('../../../../lib/stateView/document/errors/AmbiguousStartError');

describe('fetchDocumentsMethodFactory', () => {
  let contractId;
  let type;
  let options;
  let fetchDocumentsMock;
  let fetchDocumentsMethod;

  async function throwErrorAndExpectInvalidParamError(error) {
    fetchDocumentsMock.throws(error);

    let actualError;
    try {
      await fetchDocumentsMethod({ contractId, type, options });
    } catch (e) {
      actualError = e;
    }

    expect(actualError).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchDocumentsMock).to.have.been.calledOnceWith(contractId, type, options);
  }

  beforeEach(function beforeEach() {
    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    type = 'niceDocument';
    options = {};

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMethod = fetchDocumentsMethodFactory(fetchDocumentsMock);
  });

  it('should throw InvalidParamsError if Contract ID is not provided', async () => {
    let error;
    try {
      await fetchDocumentsMethod({});
    } catch (e) {
      error = e;
    }

    expect(error).to.be.an.instanceOf(InvalidParamsError);

    expect(fetchDocumentsMock).to.have.not.been.called();
  });

  it('should throw InvalidParamsError if InvalidWhereError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidWhereError());
  });

  it('should throw InvalidParamsError if InvalidOrderByError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidOrderByError());
  });

  it('should throw InvalidParamsError if InvalidLimitError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidLimitError());
  });

  it('should throw InvalidParamsError if InvalidStartAtError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidStartAtError());
  });

  it('should throw InvalidParamsError if InvalidStartAfterError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new InvalidStartAfterError());
  });

  it('should throw InvalidParamsError if AmbiguousStartError is thrown', async () => {
    await throwErrorAndExpectInvalidParamError(new AmbiguousStartError());
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
