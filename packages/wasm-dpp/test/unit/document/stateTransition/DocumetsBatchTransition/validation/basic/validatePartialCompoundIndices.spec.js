const validatePartialCompoundIndicesJs = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/basic/validatePartialCompoundIndices');
const InconsistentCompoundIndexDataErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/InconsistentCompoundIndexDataError');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');
const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let Identifier;
let validatePartialCompoundIndices;
let InconsistentCompoundIndexDataError;
let ValidationResult;

describe('validatePartialCompoundIndices', () => {
  let documentsJs;
  let rawDocumentTransitions;
  let dataContractJs;
  let dataContract;
  let ownerIdJs;
  let ownerId;


  beforeEach(async () => {
    ({
      DataContract, Identifier, Document,
      validatePartialCompoundIndices,
      ValidationResult,
      // Errors:
      InconsistentCompoundIndexDataError,
    } = await loadWasmDpp());

    dataContractJs = getContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());
    ownerIdJs = dataContractJs.getOwnerId();
    ownerId = Identifier.from(ownerIdJs.toBuffer());
  });

  it('should return invalid result if compound index contains not all fields', () => {
    const document = getDocumentsFixture(dataContractJs)[9];
    document.set('lastName', undefined);

    documentsJs = [document];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndicesJs(ownerIdJs, rawDocumentTransitions, dataContractJs);

    expectValidationError(result, InconsistentCompoundIndexDataErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1021);

    const { optionalUniqueIndexedDocument } = dataContractJs.getDocuments();

    expect(error.getIndexedProperties()).to.deep.equal(
      optionalUniqueIndexedDocument.indices[1].properties.map((i) => Object.keys(i)[0]),
    );

    expect(error.getDocumentType()).to.equal('optionalUniqueIndexedDocument');
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

  it('should return valid result if compound index contains no fields', () => {
    const document = getDocumentsFixture(dataContractJs)[8];
    document.setData({});

    documentsJs = [document];

    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndicesJs(ownerIdJs, rawDocumentTransitions, dataContractJs);

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
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

  it('should return valid result if compound index contains all fields', () => {
    documentsJs = [getDocumentsFixture(dataContractJs)[8]];
    rawDocumentTransitions = getDocumentTransitionsFixture({
      create: documentsJs,
    }).map((documentTransition) => documentTransition.toObject());

    const result = validatePartialCompoundIndicesJs(ownerIdJs, rawDocumentTransitions, dataContractJs);

    expect(result).to.be.an.instanceOf(ValidationResultJs);
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
