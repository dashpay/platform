const validatePartialCompoundIndices = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/validation/basic/validatePartialCompoundIndices');
const InconsistentCompoundIndexDataError = require('../../../../../../../lib/errors/consensus/basic/document/InconsistentCompoundIndexDataError');

const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

describe('validatePartialCompoundIndices', () => {
  let documents;
  let rawDocumentTransitions;
  let dataContract;
  let ownerId;

  beforeEach(() => {
    dataContract = getContractFixture();
    ownerId = dataContract.getOwnerId();
  });

  it('should return invalid result if compound index contains not all fields', () => {
    const document = getDocumentsFixture(dataContract)[9];
    document.set('lastName', undefined);

    documents = [document];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(ownerId, rawDocumentTransitions, dataContract);

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

    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(ownerId, rawDocumentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if compound index contains all fields', () => {
    documents = [getDocumentsFixture(dataContract)[8]];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(ownerId, rawDocumentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
