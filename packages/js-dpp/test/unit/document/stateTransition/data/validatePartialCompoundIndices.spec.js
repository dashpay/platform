const validatePartialCompoundIndices = require('../../../../../lib/document/stateTransition/validation/data/validatePartialCompoundIndices');
const InconsistentCompoundIndexDataError = require('../../../../../lib/errors/InconsistentCompoundIndexDataError');

const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');
const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

describe('validatePartialCompoundIndices', () => {
  let documents;
  let documentTransitions;
  let dataContract;
  let ownerId;

  beforeEach(() => {
    dataContract = getContractFixture();
    ownerId = '5bGvpuiXVW3yK8np1u51Y2LFk2WCvztpa8yYy6VJpguc';
  });

  it('should return invalid result if compound index contains not all fields', () => {
    const document = getDocumentsFixture(dataContract)[9];
    document.set('lastName', undefined);

    documents = [document];
    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    const result = validatePartialCompoundIndices(ownerId, documentTransitions, dataContract);

    expectValidationError(result, InconsistentCompoundIndexDataError);

    const { optionalUniqueIndexedDocument } = dataContract.getDocuments();
    const [error] = result.getErrors();

    expect(error.getIndexDefinition()).to.deep.equal(
      optionalUniqueIndexedDocument.indices[1],
    );
    expect(error.getDocumentType()).to.equal('optionalUniqueIndexedDocument');
  });

  it('should return valid result if compound index contains no fields', () => {
    const document = getDocumentsFixture(dataContract)[8];
    document.setData({ });

    documents = [document];

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    const result = validatePartialCompoundIndices(ownerId, documentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if compound index contains all fields', () => {
    documents = [getDocumentsFixture(dataContract)[8]];
    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    const result = validatePartialCompoundIndices(ownerId, documentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
