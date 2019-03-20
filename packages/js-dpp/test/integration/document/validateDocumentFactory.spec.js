const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const Document = require('../../../lib/document/Document');
const validateDocumentFactory = require('../../../lib/document/validateDocumentFactory');
const enrichDPContractWithBaseDocument = require('../../../lib/document/enrichDPContractWithBaseDocument');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const MissingDocumentTypeError = require('../../../lib/errors/MissingDocumentTypeError');
const MissingDocumentActionError = require('../../../lib/errors/MissingDocumentActionError');
const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentScopeIdError = require('../../../lib/errors/InvalidDocumentScopeIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const JsonSchemaError = require('../../../lib/errors/JsonSchemaError');

const originalDocumentBaseSchema = require('../../../schema/base/document');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

describe('validateDocumentFactory', () => {
  let dpContract;
  let rawDocuments;
  let rawDocument;
  let validateDocument;
  let validator;
  let documentBaseSchema;

  beforeEach(function beforeEach() {
    const ajv = new Ajv();

    validator = new JsonSchemaValidator(ajv);
    this.sinonSandbox.spy(validator, 'validate');

    dpContract = getDPContractFixture();

    validateDocument = validateDocumentFactory(
      validator,
      enrichDPContractWithBaseDocument,
    );

    rawDocuments = getDocumentsFixture().map(o => o.toJSON());
    [rawDocument] = rawDocuments;

    documentBaseSchema = JSON.parse(
      JSON.stringify(originalDocumentBaseSchema),
    );
  });

  describe('Base schema', () => {
    describe('$type', () => {
      it('should be present', () => {
        delete rawDocument.$type;

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(
          result,
          MissingDocumentTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawDocument()).to.equal(rawDocument);
      });

      it('should be defined in DP Contract', () => {
        rawDocument.$type = 'undefinedDocument';

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(
          result,
          InvalidDocumentTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getType()).to.equal('undefinedDocument');
      });

      it('should throw an error if getDocumentSchemaRef throws error', function it() {
        const someError = new Error();

        this.sinonSandbox.stub(dpContract, 'getDocumentSchemaRef').throws(someError);

        let error;
        try {
          validateDocument(rawDocument, dpContract);
        } catch (e) {
          error = e;
        }

        expect(error).to.equal(someError);

        expect(dpContract.getDocumentSchemaRef).to.have.been.calledOnce();
      });
    });

    describe('$action', () => {
      it('should be present', () => {
        delete rawDocument.$action;

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(
          result,
          MissingDocumentActionError,
        );

        const [error] = result.getErrors();

        expect(error.getRawDocument()).to.equal(rawDocument);
      });

      it('should be a number', () => {
        rawDocument.$action = 'string';

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$action');
        expect(error.keyword).to.equal('type');
      });

      it('should be defined enum', () => {
        rawDocument.$action = 3;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$action');
        expect(error.keyword).to.equal('enum');
      });
    });

    describe('$rev', () => {
      it('should return an error if $rev is not present', () => {
        delete rawDocument.$rev;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$rev');
      });

      it('should be a number', () => {
        rawDocument.$rev = 'string';

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('type');
      });

      it('should be an integer', () => {
        rawDocument.$rev = 1.1;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('multipleOf');
      });

      it('should be greater or equal to zero', () => {
        rawDocument.$rev = -1;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('minimum');
      });
    });

    describe('$scope', () => {
      it('should be present', () => {
        delete rawDocument.$scope;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$scope');
      });

      it('should be a string', () => {
        rawDocument.$scope = 1;

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('type');
      });

      it('should be no less than 64 chars', () => {
        rawDocument.$scope = '86b273ff';

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 64 chars', () => {
        rawDocument.$scope = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDocument(rawDocument, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('maxLength');
      });
    });

    describe('$scopeId', () => {
      it('should be present', () => {
        delete rawDocument.$scopeId;

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('');
        expect(jsonError.keyword).to.equal('required');
        expect(jsonError.params.missingProperty).to.equal('$scopeId');

        expect(scopeError).to.be.an.instanceOf(InvalidDocumentScopeIdError);
        expect(scopeError.getRawDocument()).to.equal(rawDocument);
      });

      it('should be a string', () => {
        rawDocument.$scopeId = 1;

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('type');

        expect(scopeError).to.be.an.instanceOf(InvalidDocumentScopeIdError);
        expect(scopeError.getRawDocument()).to.equal(rawDocument);
      });

      it('should be no less than 34 chars', () => {
        rawDocument.$scopeId = '86b273ff';

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('minLength');

        expect(scopeError).to.be.an.instanceOf(InvalidDocumentScopeIdError);
        expect(scopeError.getRawDocument()).to.equal(rawDocument);
      });

      it('should be no longer than 34 chars', () => {
        rawDocument.$scopeId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('maxLength');

        expect(scopeError).to.be.an.instanceOf(InvalidDocumentScopeIdError);
        expect(scopeError.getRawDocument()).to.equal(rawDocument);
      });

      it('should be valid entropy', () => {
        rawDocument.$scopeId = '86b273ff86b273ff86b273ff86b273ff86';

        const result = validateDocument(rawDocument, dpContract);

        expectValidationError(result, InvalidDocumentScopeIdError);

        const [error] = result.getErrors();

        expect(error).to.be.an.instanceOf(InvalidDocumentScopeIdError);
        expect(error.getRawDocument()).to.equal(rawDocument);
      });
    });
  });

  describe('DP Contract schema', () => {
    it('should return an error if the first document is not valid against DP Contract', () => {
      rawDocuments[0].name = 1;

      const result = validateDocument(rawDocuments[0], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('type');
    });

    it('should return an error if the second document is not valid against DP Contract', () => {
      rawDocuments[1].undefined = 1;

      const result = validateDocument(rawDocuments[1], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('additionalProperties');
    });
  });

  it('should validate against base Document schema if $action is DELETE', () => {
    delete rawDocument.name;
    rawDocument.$action = Document.ACTIONS.DELETE;

    const result = validateDocument(rawDocument, dpContract);

    expect(validator.validate).to.have.been.calledOnceWith(documentBaseSchema, rawDocument);
    expect(result.getErrors().length).to.equal(0);
  });

  it('should throw validation error if additional fields are defined and $action is DELETE', () => {
    rawDocument.$action = Document.ACTIONS.DELETE;

    const result = validateDocument(rawDocument, dpContract);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('additionalProperties');
  });

  it('should return valid response is a document is valid', () => {
    const result = validateDocument(rawDocument, dpContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
