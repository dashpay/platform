const validateQueryFactory = require('../../../../lib/document/query/validateQueryFactory');
const ValidationResult = require('../../../../lib/document/query/ValidationResult');

const ConflictingConditionsError = require('../../../../lib/document/query/errors/ConflictingConditionsError');
const NestedSystemFieldError = require('../../../../lib/document/query/errors/NestedSystemFieldError');
const NestedElementMatchError = require('../../../../lib/document/query/errors/NestedElementMatchError');
const DuplicateSortingFieldError = require('../../../../lib/document/query/errors/DuplicateSortingFieldError');

const typesTestCases = {
  number: {
    type: 'number',
    value: 1,
  },
  boolean: {
    type: 'boolean',
    value: true,
  },
  string: {
    type: 'string',
    value: 'test',
  },
  null: {
    type: 'null',
    value: null,
  },
  undefined: {
    type: 'undefined',
    value: undefined,
  },
  function: {
    type: 'function',
    value: () => {},
  },
  object: {
    type: 'object',
    value: {},
  },
};

const notObjectTestCases = [
  typesTestCases.number,
  typesTestCases.boolean,
  typesTestCases.string,
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.function,
];

const notArrayTestCases = [
  typesTestCases.number,
  typesTestCases.boolean,
  typesTestCases.string,
  typesTestCases.null,
  typesTestCases.object,
  typesTestCases.function,
];

const nonScalarTestCases = [
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.function,
  typesTestCases.object,
];

const scalarTestCases = [
  typesTestCases.number,
  typesTestCases.string,
  typesTestCases.boolean,
];

const nonStringTestCases = [
  typesTestCases.number,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.object,
  typesTestCases.function,
];

const nonNumberTestCases = [
  typesTestCases.string,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.object,
  typesTestCases.function,
];

const nonNumberAndUndefinedTestCases = [
  typesTestCases.string,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.object,
  typesTestCases.function,
];

const validFieldNameTestCases = [
  'a',
  'a.b',
  'a.b.c',
  'array.element',
  'a.0',
  'a.0.b',
  'a_._b',
  'a-b.c_',
  '$id',
  '$ownerId',
  '$createdAt',
  '$updatedAt',
];

const invalidFieldNameTestCases = [
  '$a',
  '$#1321',
  'a...',
  '.a',
  'a.b.c.',
];

describe('validateQueryFactory', () => {
  let findConflictingConditionsStub;
  let validateQuery;
  let getIndexedFieldsFromDocumentSchemaStub;
  let validateIndexedFieldsStub;
  let validateOrderByFieldsStub;
  let documentSchema;

  beforeEach(function beforeEach() {
    findConflictingConditionsStub = this.sinon.stub().returns([]);
    getIndexedFieldsFromDocumentSchemaStub = this.sinon.stub().returns([]);
    validateIndexedFieldsStub = this.sinon.stub().returns([]);
    validateOrderByFieldsStub = this.sinon.stub().returns([]);
    documentSchema = {};

    validateQuery = validateQueryFactory(
      findConflictingConditionsStub,
      getIndexedFieldsFromDocumentSchemaStub,
      validateIndexedFieldsStub,
      validateOrderByFieldsStub,
    );
  });

  it('should return valid result if empty query is specified', () => {
    const result = validateQuery({}, {});

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  notObjectTestCases.forEach(({ type, value }) => {
    it(`should return invalid result if query is a ${type}`, () => {
      const result = validateQuery(value, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].keyword).to.be.equal('type');
      expect(result.errors[0].params.type).to.be.equal('object');
    });
  });

  it('should return valid result when some valid sample query is passed', () => {
    const result = validateQuery({ where: [['a', '>', 1]] }, documentSchema);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  describe('where', () => {
    notArrayTestCases.forEach(({ type, value }) => {
      it(`should return invalid result if "where" is not an array, but ${type}`, () => {
        const result = validateQuery({ where: value }, documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();
        expect(result.errors[0].dataPath).to.be.equal('.where');
        expect(result.errors[0].keyword).to.be.equal('type');
        expect(result.errors[0].params.type).to.be.equal('array');
      });
    });

    it('should return invalid result if "where" is an empty array', () => {
      const result = validateQuery({ where: [] }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.where');
      expect(result.errors[0].keyword).to.be.equal('minItems');
      expect(result.errors[0].params.limit).to.be.equal(1);
    });

    it('should return invalid result if "where" contains more than 10 conditions', () => {
      const where = Array(11).fill(['a', '<', 1]);

      const result = validateQuery({ where }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.where');
      expect(result.errors[0].keyword).to.be.equal('maxItems');
      expect(result.errors[0].params.limit).to.be.equal(10);
    });

    it('should return invalid result if "where" contains conflicting conditions', () => {
      findConflictingConditionsStub.returns([['a', ['<', '>']]]);

      const result = validateQuery({
        where: [
          ['a', '<', 1],
          ['a', '>', 1],
        ],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0]).to.be.an.instanceOf(ConflictingConditionsError);
      expect(result.errors[0].getField()).to.be.equal('a');
      expect(result.errors[0].getOperators()).to.be.deep.equal(['<', '>']);
    });

    describe('condition', () => {
      describe('field', () => {
        it('should return valid result if condition contains "$id" field', () => {
          const result = validateQuery({ where: [['$id', '==', 'idvalue']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return valid result if condition contains "$ownerId" field', () => {
          const result = validateQuery({ where: [['$ownerId', '==', 'userid']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return valid result if condition contains "$createdAt" field', () => {
          const result = validateQuery({ where: [['$createdAt', '==', Date.now()]] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return valid result if condition contains "$updatedAt" field', () => {
          const result = validateQuery({ where: [['$updatedAt', '==', Date.now()]] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return valid result if condition contains top-level field', () => {
          const result = validateQuery({ where: [['a', '==', '1']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return valid result if condition contains nested path field', () => {
          const result = validateQuery({ where: [['a.b', '==', '1']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();
        });

        it('should return invalid result if field name is more than 255 characters long', () => {
          const fieldName = 'a'.repeat(255);

          let result = validateQuery({ where: [[fieldName, '==', '1']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.true();

          const longFieldName = 'a'.repeat(256);

          result = validateQuery({ where: [[longFieldName, '==', '1']] }, documentSchema);

          expect(result).to.be.instanceOf(ValidationResult);
          expect(result.isValid()).to.be.false();

          expect(result.errors[0].dataPath).to.be.equal('.where[0][0]');
          expect(result.errors[0].keyword).to.be.equal('maxLength');
          expect(result.errors[0].params.limit).to.be.equal(255);
        });

        invalidFieldNameTestCases.forEach((fieldName) => {
          it(`should return invalid result if field name contains restricted symbols: ${fieldName}`, () => {
            const result = validateQuery({ where: [[fieldName, '==', '1']] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0].dataPath).to.be.equal('.where[0][0]');
            expect(result.errors[0].keyword).to.be.equal('pattern');
            expect(result.errors[0].params.pattern).to.be.equal(
              '^(\\$id|\\$ownerId|[a-zA-Z0-9-_]|[a-zA-Z0-9-_]+(.[a-zA-Z0-9-_]+)+?)$',
            );
          });
        });
      });

      it('should return invalid result if condition array has less than 3 elements (field, operator, value)', () => {
        const result = validateQuery({ where: [['a', '==']] }, documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();

        expect(result.errors[0].dataPath).to.be.equal('.where[0]');
        expect(result.errors[0].keyword).to.be.equal('minItems');
        expect(result.errors[0].params.limit).to.be.equal(3);
      });

      it('should return invalid result if condition array has more than 3 elements (field, operator, value)', () => {
        const result = validateQuery({ where: [['a', '==', '1', '2']] }, documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();
        expect(result.errors[0].dataPath).to.be.equal('.where[0]');
        expect(result.errors[0].keyword).to.be.equal('maxItems');
        expect(result.errors[0].params.limit).to.be.equal(3);
      });

      describe('operators', () => {
        describe('comparisons', () => {
          it('should return invalid result if condition contains invalid comparison operator', () => {
            const operators = ['<', '<=', '==', '>', '>='];

            operators.forEach((operator) => {
              const result = validateQuery({ where: [['a', operator, '1']] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
            const result = validateQuery({ where: [['a', '===', '1']] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();

            expect(result.errors[7].dataPath).to.be.equal('.where[0]');
            expect(result.errors[7].keyword).to.be.equal('oneOf');
          });

          it('should return valid result if "<" operator used with a numeric value', () => {
            const result = validateQuery({ where: [['a', '<', 1]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return valid result if "<" operator used with a string value', () => {
            const result = validateQuery({ where: [['a', '<', 'test']] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return invalid result if "<" operator used with a string value longer than 512 chars', () => {
            const longString = 't'.repeat(512);

            let result = validateQuery({ where: [['a', '<', longString]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();

            const veryLongString = 't'.repeat(513);

            result = validateQuery({ where: [['a', '<', veryLongString]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[0].keyword).to.be.equal('maxLength');
            expect(result.errors[0].params.limit).to.be.equal(512);
          });

          it('should return valid result if "<" operator used with a boolean value', () => {
            const result = validateQuery({ where: [['a', '<', true]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "<" operator used with a not scalar value, but ${type}`, () => {
              const result = validateQuery({ where: [['a', '<', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[0].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[0].keyword).to.be.equal('type');
              expect(result.errors[0].params.type).to.be.equal('string');
              expect(result.errors[1].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[1].keyword).to.be.equal('type');
              expect(result.errors[1].params.type).to.be.equal('number');
              expect(result.errors[2].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[2].keyword).to.be.equal('type');
              expect(result.errors[2].params.type).to.be.equal('boolean');
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "<" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({ where: [['a', '<', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "<=" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({ where: [['a', '<=', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "==" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({ where: [['a', '==', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if ">=" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({ where: [['a', '<=', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if ">=" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({ where: [['a', '>', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });
        });

        describe('timestamps', () => {
          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if $createdAt timestamp used with ${type} value`, () => {
              const result = validateQuery({ where: [['$createdAt', '>', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[1].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[1].keyword).to.be.equal('type');
              expect(result.errors[1].params.type).to.be.equal('integer');
            });
          });

          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if $updatedAt timestamp used with ${type} value`, () => {
              const result = validateQuery({ where: [['$updatedAt', '>', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[1].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[1].keyword).to.be.equal('type');
              expect(result.errors[1].params.type).to.be.equal('integer');
            });
          });

          it('should return valid result if condition contains "$createdAt" field', () => {
            const result = validateQuery({ where: [['$createdAt', '==', Date.now()]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return valid result if condition contains "$updatedAt" field', () => {
            const result = validateQuery({ where: [['$updatedAt', '==', Date.now()]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });
        });

        describe('in', () => {
          it('should return valid result if "in" operator used with an array value', () => {
            const result = validateQuery({ where: [['a', 'in', [1, 2]]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          notArrayTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "in" operator used with not an array value, but ${type}`, () => {
              const result = validateQuery({ where: [['a', 'in', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[2].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[2].keyword).to.be.equal('type');
              expect(result.errors[2].params.type).to.be.equal('array');
            });
          });

          it('should return invalid result if "in" operator used with an empty array value', () => {
            const result = validateQuery({ where: [['a', 'in', []]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[2].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[2].keyword).to.be.equal('minItems');
            expect(result.errors[2].params.limit).to.be.equal(1);
          });

          it('should return invalid result if "in" operator used with an array value which contains more than 100'
            + ' elements', () => {
            const arr = [];

            for (let i = 0; i < 100; i++) {
              arr.push(i);
            }

            let result = validateQuery({ where: [['a', 'in', arr]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();

            arr.push(101);

            result = validateQuery({ where: [['a', 'in', arr]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[2].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[2].keyword).to.be.equal('maxItems');
            expect(result.errors[2].params.limit).to.be.equal(100);
          });

          it('should return invalid result if "in" operator used with an array which contains not unique elements', () => {
            const arr = [1, 1];
            const result = validateQuery({ where: [['a', 'in', arr]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[2].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[2].keyword).to.be.equal('uniqueItems');
            expect(result.errors[2].message).to.be.equal('should NOT have duplicate items (items ## 0 and 1 are identical)');
          });

          it('should return invalid results if condition contains empty arrays', () => {
            const arr = [[], []];
            const result = validateQuery({ where: [['a', 'in', arr]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
          });
        });

        describe('startsWith', () => {
          it('should return valid result if "startsWith" operator used with a string value', () => {
            const result = validateQuery({ where: [['a', 'startsWith', 'b']] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return invalid result if "startsWith" operator used with an empty string value', () => {
            const result = validateQuery({ where: [['a', 'startsWith', '']] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[3].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[3].keyword).to.be.equal('minLength');
            expect(result.errors[3].params.limit).to.be.equal(1);
          });

          it('should return invalid result if "startsWith" operator used with a string value which is more than 255'
            + ' chars long', () => {
            const value = 'b'.repeat(256);
            const result = validateQuery({ where: [['a', 'startsWith', value]] }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[3].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[3].keyword).to.be.equal('maxLength');
            expect(result.errors[3].params.limit).to.be.equal(255);
          });

          nonStringTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "startWith" operator used with a not string value, but ${type}`, () => {
              const result = validateQuery({ where: [['a', 'startsWith', value]] }, documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[3].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[3].keyword).to.be.equal('type');
              expect(result.errors[3].params.type).to.be.equal('string');
            });
          });
        });

        describe('elementMatch', () => {
          it('should return valid result if "elementMatch" operator used with "where" conditions', () => {
            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1], ['elem', '<', 3]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return invalid result if "elementMatch" operator used with invalid "where" conditions', () => {
            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['elem', 'startsWith', 1], ['elem', '<', 3]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[11].dataPath).to.be.equal('.where[0][2][0]');
            expect(result.errors[11].keyword).to.be.equal('oneOf');
          });

          it('should return invalid result if "elementMatch" operator used with less than 2 "where" conditions', () => {
            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[4].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[4].keyword).to.be.equal('minItems');
            expect(result.errors[4].params.limit).to.be.equal(2);
          });

          it('should return invalid result if value contains conflicting conditions', () => {
            findConflictingConditionsStub.returns([['elem', ['>', '>']]]);

            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1], ['elem', '>', 1]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0]).to.be.an.instanceOf(ConflictingConditionsError);
            expect(result.errors[0].getField()).to.be.equal('elem');
            expect(result.errors[0].getOperators()).to.be.deep.equal(['>', '>']);
          });

          it('should return invalid result if $id field is specified', () => {
            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['$id', '>', 1], ['$id', '<', 3]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0]).to.be.an.instanceOf(NestedSystemFieldError);
            expect(result.errors[0].getField()).to.be.equal('$id');
          });

          it('should return invalid result if $ownerId field is specified', () => {
            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['$ownerId', '>', 1], ['$ownerId', '<', 3]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0]).to.be.an.instanceOf(NestedSystemFieldError);
            expect(result.errors[0].getField()).to.be.equal('$ownerId');
          });

          it('should return invalid result if value contains nested "elementMatch" operator', () => {
            findConflictingConditionsStub.returns([]);

            const result = validateQuery({
              where: [
                ['arr', 'elementMatch',
                  [['subArr', 'elementMatch', [
                    ['subArrElem', '>', 1], ['subArrElem', '<', 3],
                  ]], ['subArr', '<', 3]],
                ],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[0]).to.be.an.instanceOf(NestedElementMatchError);
            expect(result.errors[0].getField()).to.be.equal('subArr');
          });
        });

        describe('length', () => {
          it('should return valid result if "length" operator used with a positive numeric value', () => {
            const result = validateQuery({
              where: [
                ['arr', 'length', 2],
              ],
            }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return valid result if "length" operator used with zero', () => {
            const result = validateQuery({
              where: [
                ['arr', 'length', 0],
              ],
            }, documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();
          });

          it('should return invalid result if "length" operator used with a float numeric value', () => {
            const result = validateQuery({
              where: [
                ['arr', 'length', 1.2],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[5].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[5].keyword).to.be.equal('multipleOf');
            expect(result.errors[5].params.multipleOf).to.be.equal(1);
          });

          it('should return invalid result if "length" operator used with a NaN', () => {
            const result = validateQuery({
              where: [
                ['arr', 'length', NaN],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[5].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[5].keyword).to.be.equal('minimum');
            expect(result.errors[5].params.comparison).to.be.equal('>=');
            expect(result.errors[5].params.limit).to.be.equal(0);
          });

          it('should return invalid result if "length" operator used with a numeric value which is less than 0', () => {
            const result = validateQuery({
              where: [
                ['arr', 'length', -1],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[5].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[5].keyword).to.be.equal('minimum');
            expect(result.errors[5].params.comparison).to.be.equal('>=');
            expect(result.errors[5].params.limit).to.be.equal(0);
          });

          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "length" operator used with a ${type} instead of numeric value`, () => {
              const result = validateQuery({
                where: [
                  ['arr', 'length', value],
                ],
              },
              documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[5].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[5].keyword).to.be.equal('type');
              expect(result.errors[5].params.type).to.be.equal('number');
            });
          });
        });

        describe('contains', () => {
          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "contains" operator used with a scalar value ${type}`, () => {
              const result = validateQuery({
                where: [
                  ['arr', 'contains', value],
                ],
              },
              documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "contains" operator used with an array of scalar values ${type}`, () => {
              const result = validateQuery({
                where: [
                  ['arr', 'contains', [value]],
                ],
              },
              documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.true();
            });
          });

          it('should return invalid result if "contains" operator used with an array which has '
            + ' more than 100 elements', () => {
            const arr = [];
            for (let i = 0; i < 100; i++) {
              arr.push(i);
            }

            let result = validateQuery({
              where: [
                ['arr', 'contains', arr],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.true();

            arr.push(101);

            result = validateQuery({
              where: [
                ['arr', 'contains', arr],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[10].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[10].keyword).to.be.equal('maxItems');
            expect(result.errors[10].params.limit).to.be.equal(100);
          });

          it('should return invalid result if "contains" operator used with an empty array', () => {
            const result = validateQuery({
              where: [
                ['arr', 'contains', []],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[10].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[10].keyword).to.be.equal('minItems');
            expect(result.errors[10].params.limit).to.be.equal(1);
          });

          it('should return invalid result if "contains" operator used with an array which contains not unique'
            + ' elements', () => {
            const result = validateQuery({
              where: [
                ['arr', 'contains', [1, 1]],
              ],
            },
            documentSchema);

            expect(result).to.be.instanceOf(ValidationResult);
            expect(result.isValid()).to.be.false();
            expect(result.errors[10].dataPath).to.be.equal('.where[0][2]');
            expect(result.errors[10].keyword).to.be.equal('uniqueItems');
            expect(result.errors[10].message).to.be.equal('should NOT have duplicate items (items ## 0 and 1 are identical)');
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if used with non-scalar value ${type}`, () => {
              const result = validateQuery({
                where: [
                  ['arr', 'contains', value],
                ],
              },
              documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[6].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[6].keyword).to.be.equal('type');
              expect(result.errors[6].params.type).to.be.equal('string');
              expect(result.errors[7].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[7].keyword).to.be.equal('type');
              expect(result.errors[7].params.type).to.be.equal('number');
              expect(result.errors[8].dataPath).to.be.equal('.where[0][2]');
              expect(result.errors[8].keyword).to.be.equal('type');
              expect(result.errors[8].params.type).to.be.equal('boolean');
            });
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if used with an array of non-scalar values ${type}`, () => {
              const result = validateQuery({
                where: [
                  ['arr', 'contains', [value]],
                ],
              },
              documentSchema);

              expect(result).to.be.instanceOf(ValidationResult);
              expect(result.isValid()).to.be.false();
              expect(result.errors[10].dataPath).to.be.equal('.where[0][2][0]');
              expect(result.errors[10].keyword).to.be.equal('type');
              expect(result.errors[10].params.type).to.be.equal('string');
              expect(result.errors[11].dataPath).to.be.equal('.where[0][2][0]');
              expect(result.errors[11].keyword).to.be.equal('type');
              expect(result.errors[11].params.type).to.be.equal('number');
              expect(result.errors[12].dataPath).to.be.equal('.where[0][2][0]');
              expect(result.errors[12].keyword).to.be.equal('type');
              expect(result.errors[12].params.type).to.be.equal('boolean');
            });
          });
        });
      });
    });
  });

  describe('limit', () => {
    it('should return valid result if "limit" is a number', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        limit: 1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if "limit" is less than 1', () => {
      const where = [
        ['a', '>', 1],
      ];

      let result = validateQuery({ where, limit: 0 }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.limit');
      expect(result.errors[0].keyword).to.be.equal('minimum');
      expect(result.errors[0].params.limit).to.be.equal(1);

      result = validateQuery({ where, limit: -1 }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.limit');
      expect(result.errors[0].keyword).to.be.equal('minimum');
      expect(result.errors[0].params.comparison).to.be.equal('>=');
      expect(result.errors[0].params.limit).to.be.equal(1);
    });

    it('should return invalid result if "limit" is bigger than 100', () => {
      const where = [
        ['a', '>', 1],
      ];

      let result = validateQuery({ where, limit: 100 }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      result = validateQuery({ where, limit: 101 }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.limit');
      expect(result.errors[0].keyword).to.be.equal('maximum');
      expect(result.errors[0].params.comparison).to.be.equal('<=');
      expect(result.errors[0].params.limit).to.be.equal(100);
    });

    it('should return invalid result if "limit" is a float number', () => {
      const where = [
        ['a', '>', 1],
      ];

      const result = validateQuery({ where, limit: 1.5 }, documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.limit');
      expect(result.errors[0].keyword).to.be.equal('multipleOf');
      expect(result.errors[0].params.multipleOf).to.be.equal(1);
    });

    nonNumberAndUndefinedTestCases.forEach(({ type, value }) => {
      it(`should return invalid result if "limit" is not a number, but ${type}`, () => {
        const result = validateQuery({
          where: [
            ['a', '>', 1],
          ],
          limit: value,
        },
        documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();

        expect(result.errors[0].dataPath).to.be.equal('.limit');
        expect(result.errors[0].keyword).to.be.equal('type');
        expect(result.errors[0].params.type).to.be.equal('number');
      });
    });
  });

  describe('orderBy', () => {
    it('should return valid result if "orderBy" contains 1 sorting field', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return valid result if "orderBy" contains 2 sorting fields', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc'], ['b', 'desc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if "orderBy" is an empty array', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy');
      expect(result.errors[0].keyword).to.be.equal('minItems');
      expect(result.errors[0].params.limit).to.be.equal(1);
    });

    it('should return invalid result if the field inside an "orderBy" is an empty array', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [[]],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy[0]');
      expect(result.errors[0].keyword).to.be.equal('minItems');
      expect(result.errors[0].params.limit).to.be.equal(2);
    });

    it('should return invalid result if "orderBy" has more than 2 sorting fields', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc'], ['b', 'desc'], ['c', 'asc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy');
      expect(result.errors[0].keyword).to.be.equal('maxItems');
      expect(result.errors[0].params.limit).to.be.equal(2);
    });

    validFieldNameTestCases.forEach((fieldName) => {
      it(`should return true if "orderBy" has valid field format, ${fieldName}`, () => {
        const result = validateQuery({
          where: [
            [fieldName, '>', 1],
          ],
          orderBy: [[fieldName, 'asc']],
        },
        documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.true();
      });
    });

    invalidFieldNameTestCases.forEach((fieldName) => {
      it(`should return invalid result if "orderBy" has invalid field format, ${fieldName}`, () => {
        const result = validateQuery({
          where: [
            ['a', '>', 1],
          ],
          orderBy: [['$a', 'asc']],
        },
        documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();
        expect(result.errors[0].dataPath).to.be.equal('.orderBy[0][0]');
        expect(result.errors[0].keyword).to.be.equal('pattern');
        expect(result.errors[0].params.pattern).to.be.equal(
          '^(\\$id|\\$ownerId|\\$createdAt|\\$updatedAt|[a-zA-Z0-9-_]|[a-zA-Z0-9-_]+(.[a-zA-Z0-9-_]+)+?)$',
        );
      });
    });

    it('should return invalid result if "orderBy" has wrong direction', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'a']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy[0][1]');
      expect(result.errors[0].keyword).to.be.equal('enum');
    });

    it('should return invalid result if "orderBy" field array has less than 2 elements (field, direction)', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy[0]');
      expect(result.errors[0].keyword).to.be.equal('minItems');
      expect(result.errors[0].params.limit).to.be.equal(2);
    });

    it('should return invalid result if "orderBy" field array has more than 2 elements (field, direction)', () => {
      const result = validateQuery({
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc', 'desc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.orderBy[0]');
      expect(result.errors[0].keyword).to.be.equal('maxItems');
      expect(result.errors[0].params.limit).to.be.equal(2);
    });

    it('should return invalid result if "orderBy" contains duplicate sorting fields'
    + 'with the same direction', () => {
      const result = validateQuery({
        where: [['a', '>', 1]],
        orderBy: [['a', 'asc'], ['a', 'asc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0]).to.be.instanceOf(DuplicateSortingFieldError);
      expect(result.errors[0].getField()).to.be.equal('a');
    });

    it('should return invalid result if "orderBy" contains duplicate sorting fields'
      + ' with different directions', () => {
      const result = validateQuery({
        where: [['a', '>', 1]],
        orderBy: [['a', 'asc'], ['a', 'desc']],
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0]).to.be.instanceOf(DuplicateSortingFieldError);
      expect(result.errors[0].getField()).to.be.equal('a');
    });
  });

  describe('startAt', () => {
    it('should return valid result if "startAt" is a number', () => {
      const result = validateQuery({
        startAt: 1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    nonNumberAndUndefinedTestCases.forEach(({ type, value }) => {
      it(`should return invalid result if "startAt" is not a number, but ${type}`, () => {
        const result = validateQuery({
          startAt: value,
        },
        documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();
        expect(result.errors[0].dataPath).to.be.equal('.startAt');
        expect(result.errors[0].keyword).to.be.equal('type');
        expect(result.errors[0].params.type).to.be.equal('number');
      });
    });

    it('should return valid result if "startAt" is up to 20000', () => {
      const result = validateQuery({
        startAt: 20000,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if "startAt" less than 1', () => {
      const result = validateQuery({
        startAt: 0,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAt');
      expect(result.errors[0].keyword).to.be.equal('minimum');
      expect(result.errors[0].params.comparison).to.be.equal('>=');
      expect(result.errors[0].params.limit).to.be.equal(1);
    });

    it('should return invalid result if "startAt" more than 20000', () => {
      const result = validateQuery({
        startAt: 20001,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAt');
      expect(result.errors[0].keyword).to.be.equal('maximum');
      expect(result.errors[0].params.comparison).to.be.equal('<=');
      expect(result.errors[0].params.limit).to.be.equal(20000);
    });

    it('should return invalid result if "startAt" is not an integer', () => {
      const result = validateQuery({
        startAt: 1.1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAt');
      expect(result.errors[0].keyword).to.be.equal('multipleOf');
      expect(result.errors[0].params.multipleOf).to.be.equal(1);
    });
  });

  describe('startAfter', () => {
    it('should return invalid result if both "startAt" and "startAfter" are present', () => {
      const result = validateQuery({
        startAfter: 1,
        startAt: 1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.have.lengthOf(4);

      const [notError1, notError2, notError3, anyOfError] = result.getErrors();

      expect(notError1.schemaPath).to.equal('#/anyOf/0/not');
      expect(notError2.schemaPath).to.equal('#/anyOf/1/not');
      expect(notError3.schemaPath).to.equal('#/anyOf/2/not');
      expect(anyOfError.schemaPath).to.equal('#/anyOf');
    });

    it('should return valid result if "startAfter" is a number', () => {
      const result = validateQuery({
        startAfter: 1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    nonNumberAndUndefinedTestCases.forEach(({ type, value }) => {
      it(`should return invalid result if "startAfter" is not a number, but ${type}`, () => {
        const result = validateQuery({
          startAfter: value,
        },
        documentSchema);

        expect(result).to.be.instanceOf(ValidationResult);
        expect(result.isValid()).to.be.false();
        expect(result.errors[0].dataPath).to.be.equal('.startAfter');
        expect(result.errors[0].keyword).to.be.equal('type');
        expect(result.errors[0].params.type).to.be.equal('number');
      });
    });

    it('should return valid result if "startAfter" is up to 20000', () => {
      const result = validateQuery({
        startAfter: 20000,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if "startAfter" less than 1', () => {
      const result = validateQuery({
        startAfter: 0,
      });

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAfter');
      expect(result.errors[0].keyword).to.be.equal('minimum');
      expect(result.errors[0].params.comparison).to.be.equal('>=');
      expect(result.errors[0].params.limit).to.be.equal(1);
    });

    it('should return invalid result if "startAfter" more than 20000', () => {
      const result = validateQuery({
        startAfter: 20001,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAfter');
      expect(result.errors[0].keyword).to.be.equal('maximum');
      expect(result.errors[0].params.comparison).to.be.equal('<=');
      expect(result.errors[0].params.limit).to.be.equal(20000);
    });

    it('should return invalid result if "startAfter" is not an integer', () => {
      const result = validateQuery({
        startAfter: 1.1,
      },
      documentSchema);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      expect(result.errors[0].dataPath).to.be.equal('.startAfter');
      expect(result.errors[0].keyword).to.be.equal('multipleOf');
      expect(result.errors[0].params.multipleOf).to.be.equal(1);
    });
  });
});
