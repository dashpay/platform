const { getRE2Class } = require('@dashevo/wasm-re2');

const validateDataContractPatternsFactory = require('../../../../lib/dataContract/validation/validateDataContractPatternsFactory');
const { expectValidationError } = require(
  '../../../../lib/test/expect/expectError',
);
const IncompatibleRe2PatternError = require('../../../../lib/errors/consensus/basic/dataContract/IncompatibleRe2PatternError');

describe('validateDataContractPatternsFactory', () => {
  let validateDataContractPatterns;
  let RE2;

  before(async () => {
    RE2 = await getRE2Class();
  });

  beforeEach(() => {
    validateDataContractPatterns = validateDataContractPatternsFactory(RE2);
  });

  it('should return valid result', () => {
    const schema = {
      type: 'object',
      properties: {
        foo: { type: 'integer' },
        bar: {
          type: 'string',
          pattern: '([a-z]+)+$',
        },
      },
      required: ['foo'],
      additionalProperties: false,
    };

    const result = validateDataContractPatterns(schema);

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result on incompatible pattern', () => {
    const schema = {
      type: 'object',
      properties: {
        foo: { type: 'integer' },
        bar: {
          type: 'string',
          pattern: '^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$',
        },
      },
      required: ['foo'],
      additionalProperties: false,
    };

    const result = validateDataContractPatterns(schema);

    expectValidationError(result, IncompatibleRe2PatternError);
    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1009);
    expect(error.getPattern()).to.equal('^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$');
    expect(error.getPath()).to.equal('/properties/bar');
    expect(error.getPatternError()).to.be.instanceOf(Error);
  });
});
