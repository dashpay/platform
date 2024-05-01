const getContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');

const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let validatePartialCompoundIndices;
let InconsistentCompoundIndexDataError;
let ValidationResult;

describe.skip('validatePartialCompoundIndices', () => {
  let documentsJs;
  let rawDocumentTransitions;
  let dataContractJs;
  let dataContract;

  beforeEach(async () => {
    ({
      DataContract,
      validatePartialCompoundIndices,
      ValidationResult,
      // Errors:
      InconsistentCompoundIndexDataError,
    } = await loadWasmDpp());

    dataContractJs = getContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());
  });

  it('should return invalid result if compound index contains not all fields - Rust', () => {
    const documentJs = getDocumentsFixture(dataContractJs)[9];
    documentJs.set('lastName', undefined);

    documentsJs = [documentJs];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(rawDocumentTransitions, dataContract);

    expect(result.isValid()).is.false();

    const [error] = result.getErrors();
    expect(error).is.instanceOf(InconsistentCompoundIndexDataError);
    expect(error.getCode()).to.equal(1021);

    const { optionalUniqueIndexedDocument } = dataContractJs.getDocuments();

    expect(error.getIndexedProperties()).to.deep.equal(
      optionalUniqueIndexedDocument.indices[1].properties.map((i) => Object.keys(i)[0]),
    );

    expect(error.getDocumentType()).to.equal('optionalUniqueIndexedDocument');
  });

  it('should return valid result if compound index contains no fields - Rust', () => {
    const document = getDocumentsFixture(dataContractJs)[8];
    document.setData({});

    documentsJs = [document];

    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(rawDocumentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if compound index contains all fields - Rust', () => {
    documentsJs = [getDocumentsFixture(dataContractJs)[8]];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndices(rawDocumentTransitions, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
