const InvalidDocumentError = require('../../../../lib/document/errors/InvalidDocumentError');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

describe('InvalidDocumentError', () => {
  let rawDocument;
  let error;

  beforeEach(() => {
    error = new Error('Some error');

    const [document] = getDocumentsFixture();
    rawDocument = document.toObject();
  });

  it('should return errors', () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(errors, rawDocument);

    expect(invalidDocumentError.getErrors()).to.deep.equal(errors);
  });

  it('should return Document', async () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(errors, rawDocument);

    expect(invalidDocumentError.getRawDocument()).to.deep.equal(rawDocument);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(errors, rawDocument);

    expect(invalidDocumentError.message).to.equal(`Invalid Document: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidDocumentError = new InvalidDocumentError(errors, rawDocument);

    expect(invalidDocumentError.message).to.equal(`Invalid Document: "${error.message}" and 1 more`);
  });
});
