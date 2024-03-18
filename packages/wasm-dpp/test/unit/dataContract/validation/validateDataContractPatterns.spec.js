const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../..');

describe.skip('validateDataContractPatterns', () => {
  let DataContractValidator;
  let IncompatibleRe2PatternError;

  before(async () => {
    ({
      DataContractValidator,
      IncompatibleRe2PatternError,
    } = await loadWasmDpp());
  });

  it('should return valid result', async () => {
    const rawDataContract = (await getDataContractFixture()).toObject();
    delete rawDataContract.$defs;

    const schema = {
      type: 'object',
      properties: {
        foo: { type: 'integer' },
        bar: {
          type: 'string',
          pattern: '([a-z]+)+$',
          maxLength: 1337,
        },
      },
      required: ['foo'],
      additionalProperties: false,
    };

    rawDataContract.documents.totallyFineDocument = schema;

    const validator = new DataContractValidator();

    const result = validator.validate(rawDataContract);
    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result on incompatible pattern', async () => {
    const rawDataContract = (await getDataContractFixture()).toObject();
    delete rawDataContract.$defs;

    const schema = {
      type: 'object',
      properties: {
        foo: { type: 'integer' },
        bar: {
          type: 'string',
          pattern: '^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$',
          maxLength: 1337,
        },
      },
      required: ['foo'],
      additionalProperties: false,
    };

    rawDataContract.documents.notFineDocument = schema;

    const validator = new DataContractValidator();

    const result = validator.validate(rawDataContract);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(IncompatibleRe2PatternError);
    expect(error.getCode()).to.equal(10202);
    expect(error.getPattern()).to.equal('^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$');
    expect(error.getPath()).to.equal('/documents/notFineDocument/properties/bar');
  });
});
