const { InvalidDocumentError } = require('../../../..');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

describe('InvalidDocumentError', () => {
  let rawDocument;
  let error;

  beforeEach(async () => {
    error = new Error('Some error');

    const [document] = await getDocumentsFixture();
    rawDocument = document.toObject();
  });

  it('should return errors', () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(rawDocument, errors);

    expect(invalidDocumentError.getErrors()).to.deep.equal(errors);
  });

  it('should return Document', async () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(rawDocument, errors);

    expect(invalidDocumentError.getRawDocument()).to.deep.equal(rawDocument);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const invalidDocumentError = new InvalidDocumentError(rawDocument, errors);

    expect(invalidDocumentError.getMessage()).to.contain('Invalid document: ');
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidDocumentError = new InvalidDocumentError(errors, rawDocument);

    expect(invalidDocumentError.getMessage()).to.contain('Invalid document: ');
  });
});
