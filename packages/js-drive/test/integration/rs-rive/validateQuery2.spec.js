const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const DataContractFactory = require('@dashevo/dpp/lib/dataContract/DataContractFactory');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const DocumentFactory = require('@dashevo/dpp/lib/document/DocumentFactory');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');
const StorageResult = require('../../../lib/storage/StorageResult');

function ucFirst(string) {
  return string.charAt(0).toUpperCase() + string.slice(1);
}

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
  buffer: {
    type: 'buffer',
    value: Buffer.alloc(32),
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
  typesTestCases.buffer,
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
  typesTestCases.buffer,
];

const nonStringTestCases = [
  typesTestCases.number,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.object,
  typesTestCases.function,
  typesTestCases.buffer,
];

const nonNumberTestCases = [
  typesTestCases.string,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.undefined,
  typesTestCases.object,
  typesTestCases.function,
  typesTestCases.buffer,
];

const nonNumberAndUndefinedTestCases = [
  typesTestCases.string,
  typesTestCases.boolean,
  typesTestCases.null,
  typesTestCases.object,
  typesTestCases.function,
  typesTestCases.buffer,
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

const validOrderByOperators = {
  '>': {
    value: 42,
  },
  '<': {
    value: 42,
  },
  startsWith: {
    value: 'rt-',
  },
  in: {
    value: ['a', 'b'],
  },
};

const documentSchema = {
  documentA: {
    type: 'object',
    properties: {
      firstName: {
        type: 'string',
      },
    },
    additionalProperties: false,
    indices: [
      {
        name: 'one',
        properties: [{ $id: 'asc' }],
      },
    ],
  },
  documentB: {
    type: 'object',
    additionalProperties: false,
    properties: {
      firstName: {
        type: 'string',
      },
    },
    indices: [
      {
        properties: [{ $id: 'asc' }],
        unique: true,
      },
    ],
  },
  documentC: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'integer',
      },
      b: {
        type: 'integer',
      },
    },
    indices: [
      {
        properties: [{ a: 'asc' }, { b: 'desc' }],
      },
    ],
  },
  documentD: {
    // no index
    type: 'object',
    additionalProperties: false,
    properties: {
      firstName: {
        type: 'string',
      },
    },
  },
  documentE: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'string',
      },
      b: {
        type: 'string',
      },
    },
    indices: [
      {
        properties: [{ a: 'asc' }, { b: 'asc' }],
      },
    ],
  },
  documentF: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'integer',
      },
      b: {
        type: 'integer',
      },
      c: {
        type: 'integer',
      },
    },
    indices: [
      {
        properties: [{ a: 'asc' }, { b: 'asc' }, { c: 'asc' }],
      },
    ],
  },
  documentG: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'integer',
      },
      b: {
        type: 'integer',
      },
    },
    indices: [
      {
        properties: [{ b: 'asc' }, { a: 'asc' }],
      },
      {
        properties: [{ a: 'asc' }, { b: 'asc' }],
      },
    ],
  },
  documentH: {
    type: 'object',
    additionalProperties: false,
    properties: {
      firstName: {
        type: 'string',
      },
    },
    indices: [
      {
        properties: [{ $updatedAt: 'asc' }],
      },
    ],
  },
  documentI: {
    type: 'object',
    additionalProperties: false,
    properties: {
      firstName: {
        type: 'string',
      },
    },
    indices: [
      {
        properties: [{ $createdAt: 'asc' }],
      },
    ],
  },
  documentJ: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'integer',
      },
      b: {
        type: 'integer',
      },
      c: {
        type: 'integer',
      },
      d: {
        type: 'integer',
      },
      e: {
        type: 'integer',
      },
    },
    indices: [
      {
        name: 'index1',
        properties: [
          { a: 'asc' },
          { b: 'desc' },
          { c: 'desc' },
          { d: 'desc' },
          { e: 'desc' },
        ],
        unique: true,
      },
    ],
  },
  documentK: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'string',
      },
      b: {
        type: 'string',
      },
    },
    indices: [
      {
        properties: [{ b: 'asc' }],
      },
    ],
  },
  documentL: {
    type: 'object',
    additionalProperties: false,
    properties: {
      a: {
        type: 'integer',
      },
      b: {
        type: 'integer',
      },
      c: {
        type: 'integer',
      },
      d: {
        type: 'integer',
      },
    },
    indices: [
      {
        name: 'index1',
        properties: [
          { a: 'asc' },
          { b: 'asc' },
          { c: 'asc' },
          { d: 'asc' },
        ],
        unique: true,
      },
    ],
  },
};

for (const fieldName of validFieldNameTestCases) {
  documentSchema[`document${fieldName}`] = {
    type: 'object',
    properties: {
      [fieldName]: {
        type: 'integer',
      },
    },
    additionalProperties: false,
    indices: [
      {
        name: 'one',
        properties: [{ [fieldName]: 'asc' }],
      },
    ],
  };
}

for (const type of ['number', 'string', 'boolean', 'buffer']) {
  const properties = {
    a: {
      type,
    },
  };

  if (type === 'buffer') {
    properties.a.type = 'array';
    properties.a.byteaArray = true;
  }

  documentSchema[`document${ucFirst(type)}`] = {
    type: 'object',
    properties,
    additionalProperties: false,
    indices: [
      {
        name: 'one',
        properties: [{ a: 'asc' }],
      },
    ],
  };
}

documentSchema.documentBig = {
  type: 'object',
  properties: Array(256).fill().map((v, i) => `a${i}`).reduce((res, key) => {
    res[key] = {
      type: 'integer',
    };

    return res;
  }, {}),
  additionalProperties: false,
  indices: Array(256).fill().map((v, i) => ({
    properties: [{ [`a${i}`]: 'asc' }],
  })),
};

describe('validate RS Drive query', () => {
  let container;
  let dataContract;
  let documentRepository;
  let dataContractRepository;

  before(async () => {
    container = await createTestDIContainer();

    documentRepository = container.resolve('documentRepository');
    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    dataContractRepository = container.resolve('dataContractRepository');

    await createInitialStateStructure();

    // const validPropertyName = 'a'.repeat(255);
    // const longPropertyName = 'a'.repeat(256);

    const factory = new DataContractFactory(createDPPMock(), () => {});
    const ownerId = generateRandomIdentifier();
    dataContract = factory.create(ownerId, documentSchema);
    await dataContractRepository.store(dataContract);
  });

  after(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should return valid result if empty query is specified', async () => {
    const query = {};

    const result = await documentRepository.find(dataContract, 'documentA', query);

    expect(result).to.deep.equal({});
  });

  notObjectTestCases.forEach(({ type, value: query }) => {
    it(`should return invalid result if query is a ${type}`, async () => {
      try {
        await documentRepository.find(dataContract, 'documentA', query);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
      }
    });
  });

  it('should return valid result when some valid sample query is passed', async () => {
    const result = await documentRepository.find(dataContract, 'documentA', { where: [['$id', '==', generateRandomIdentifier()]] });

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.isEmpty()).to.be.true();
  });

  describe('where', () => {
    notArrayTestCases.forEach(({ type, value: query }) => {
      it(`should return invalid result if "where" is not an array, but ${type}`, async () => {
        try {
          await documentRepository.find(dataContract, 'documentA', { where: query });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
        }
      });
    });

    it('should return invalid result if "where" is an empty array', async () => {
      try {
        await documentRepository.find(dataContract, 'documentA', { where: [] });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
      }
    });

    it('should return invalid result if "where" contains more than 10 conditions', async () => {
      const where = Array(11).fill(['a', '<', 1]);
      try {
        await documentRepository.find(dataContract, 'documentA', { where });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: multiple range clauses error: there can only be at most 2 range clauses that must be on the same field');
      }
    });

    it('should return invalid result if "where" contains conflicting conditions', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '<', 1],
            ['a', '>', 1],
          ],
          orderBy: [['a', 'asc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
      }
    });

    it('should return invalid result if number of properties queried does not match number of indexed ones minus 2', async () => {
      try {
        await documentRepository.find(dataContract, 'documentL', {
          where: [
            ['a', '==', 1],
          ],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: query is too far from index: query must better match an existing index');
      }
    });

    describe('condition', () => {
      describe('property', () => {
        it('should return valid result if condition contains "$id" field', async () => {
          const result = await documentRepository.find(dataContract, 'documentB', {
            where:
              [['$id', '==', generateRandomIdentifier()]],
          });

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.isEmpty()).to.be.true();
        });

        it('should return valid result if condition contains top-level field', async () => {
          const result = await documentRepository.find(dataContract, 'documentE', {
            where: [
              ['a', '==', '1'],
            ],
          });

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.isEmpty()).to.be.true();
        });

        it.skip('should return valid result if condition contains nested path field', async () => {
          const result = await documentRepository.find(dataContract, 'documentD', {
            where:
              [['a.b', '==', '1']],
          });

          expect(result).to.be.instanceOf(StorageResult);
        });

        it('should return invalid result if property is not specified in document indices', async () => {
          try {
            await documentRepository.find(dataContract, 'documentD', {
              where: [
                ['a', '==', '1'],
              ],
            });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
          }
        });

        it.skip('should return invalid result if field name is more than 255 characters long', async () => {
          // let result = await documentRepository.find(dataContract, 'documentF', {
          //   where: [
          //     [[validPropertyName, '==', '1']],
          //   ],
          // });
          //
          // expect(result).to.be.instanceOf(ValidationResult);
          // expect(result.isValid()).to.be.true();
          //
          // result = await documentRepository.find(dataContract, 'documentF', {
          //   where: [
          //     [[longPropertyName, '==', '1']],
          //   ],
          // });
          //
          // expect(result).to.be.instanceOf(ValidationResult);
          // expect(result.isValid()).to.be.false();
          //
          // expect(result.errors[0].instancePath).to.be.equal('/where/0/0');
          // expect(result.errors[0].keyword).to.be.equal('maxLength');
          // expect(result.errors[0].params.limit).to.be.equal(255);
        });

        invalidFieldNameTestCases.forEach((fieldName) => {
          it(`should return invalid result if field name contains restricted symbols: ${fieldName}`, async () => {
            try {
              await documentRepository.find(dataContract, 'documentA', {
                where:
                [[fieldName, '==', '1']],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
            }
          });
        });
      });

      it('should return invalid result if condition array has less than 3 elements (field, operator, value)', async () => {
        try {
          await documentRepository.find(dataContract, 'documentA', {
            where:
              [['a', '==']],

          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          expect(e.message).to.equal('Invalid query: invalid where clause components error: where clauses should have at most 3 components');
        }
      });

      it('should return invalid result if condition array has more than 3 elements (field, operator, value)', async () => {
        try {
          await documentRepository.find(dataContract, 'documentA', {
            where: [
              [['a', '==', '1', '2']],
            ],
          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          expect(e.message).to.equal('Invalid query: invalid where clause components error: where clauses should have at most 3 components');
        }
      });

      describe('operators', () => {
        describe('comparisons', () => {
          it('should return invalid result if condition contains invalid comparison operator', async () => {
            const operators = ['<', '<=', '==', '>', '>='];

            const promises = operators.map(async (operator) => {
              const query = { where: [['a', operator, '1']] };
              if (operator !== '==') {
                query.orderBy = [['a', 'asc']];
              }

              try {
                await documentRepository.find(dataContract, 'documentE', query);

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });

            await Promise.all(promises);

            try {
              await documentRepository.find(dataContract, 'documentString', { where: [['a', '===', '1']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return valid result if "<" operator used with a numeric value', async () => {
            const result = await documentRepository.find(dataContract, 'documentNumber', { where: [['a', '<', 1]], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return valid result if "<" operator used with a string value', async () => {
            const result = await documentRepository.find(dataContract, 'documentE', { where: [['a', '<', 'test']], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it.skip('should return invalid result if "<" operator used with a string value longer than 1024 chars', async () => {
            const longString = 't'.repeat(1024);

            try {
              await documentRepository.find(dataContract, 'documentString', { where: [['a', '<', longString]] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
            }

            const veryLongString = 't'.repeat(1025);

            try {
              await documentRepository.find(dataContract, 'documentString', { where: [['a', '<', veryLongString]] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
            }
          });

          it('should return valid result if "<" operator used with a boolean value', async () => {
            const result = await documentRepository.find(dataContract, 'documentBoolean', { where: [['a', '<', true]], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "<" operator used with a not scalar value, but ${type}`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentNumber', { where: [['a', '<', value]], orderBy: [['a', 'asc']] });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
              }
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "<" operator used with a scalar value ${type}`, async () => {
              const docType = `document${ucFirst(type)}`;

              const result = await documentRepository.find(dataContract, docType, { where: [['a', '<', value]], orderBy: [['a', 'asc']] });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "<=" operator used with a scalar value ${type}`, async () => {
              const result = await documentRepository.find(dataContract, `document${ucFirst(type)}`, { where: [['a', '<=', value]], orderBy: [['a', 'asc']] });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "==" operator used with a scalar value ${type}`, async () => {
              const result = await documentRepository.find(dataContract, `document${ucFirst(type)}`, { where: [['a', '==', value]] });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if ">=" operator used with a scalar value ${type}`, async () => {
              const result = await documentRepository.find(dataContract, `document${ucFirst(type)}`, { where: [['a', '>=', value]], orderBy: [['a', 'asc']] });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if ">" operator used with a scalar value ${type}`, async () => {
              const result = await documentRepository.find(dataContract, `document${ucFirst(type)}`, { where: [['a', '>', value]], orderBy: [['a', 'asc']] });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          ['>', '<', '<=', '>='].forEach((operator) => {
            it(`should return invalid results if "${operator}" used not in the last 2 where conditions`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentNumber', {
                  where: [
                    ['a', operator, 1],
                    ['a', 'startsWith', 'rt-'],
                    ['a', 'startsWith', 'r-'],
                  ],
                  orderBy: [['a', 'asc']],
                });
                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('Invalid query: range clauses not groupable error: clauses are not groupable');
              }
            });
          });

          describe('ranges', () => {
            ['>', '<', '<=', '>='].forEach((operator) => {
              it(`should return invalid result if ${operator} operator used with another range operator`, async () => {
                const promises = ['>', '<', '>=', '<=', 'startsWith'].map(async (additionalOperator) => {
                  const query = { where: [['a', operator, '1'], ['b', additionalOperator, 'a']] };

                  try {
                    await documentRepository.find(dataContract, 'documentE', query);

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    // expect(e.message).to.equal('Invalid query: multiple range clauses error: all ranges must be on same field');
                  }
                });

                await Promise.all(promises);
              });
            });

            it('should return invalid result if "in" operator is used before last two indexed conditions', async () => {
              const query = { where: [['a', 'in', [1, 2]]] };

              try {
                await documentRepository.find(dataContract, 'documentF', query);

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                // TODO is it correct ??????
                expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
              }
            });

            ['>', '<', '>=', '<='].forEach((operator) => {
              it(`should return invalid result if ${operator} operator is used before "=="`, async () => {
                const query = { where: [['a', operator, 2], ['b', '==', 1]], orderBy: [['a', 'asc']] };

                try {
                  await documentRepository.find(dataContract, 'documentF', query);
                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                  // TODO is it correct?
                  expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
                }
              });
            });

            ['>', '<', '>=', '<='].forEach((operator) => {
              it(`should return invalid result if ${operator} operator is used before "in"`, async () => {
                const query = { where: [['a', operator, 2], ['b', 'in', [1, 2]]], orderBy: [['a', 'asc'], ['b', 'asc']] };

                try {
                  await documentRepository.find(dataContract, 'documentG', query);
                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                  expect(e.message).to.equal('');
                }
              });
            });

            it('should return invalid result if "in" or range operators are not in orderBy', async () => {
              const query = {
                where: [
                  ['a', '==', 1],
                  ['b', '>', 1],
                ],
                orderBy: [['b', 'asc']],
              };

              delete query.orderBy;

              try {
                await documentRepository.find(dataContract, 'documentF', query);
                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
              }
            });
          });
        });

        describe('timestamps', () => {
          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if $createdAt timestamp used with ${type} value`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentI', { where: [['$createdAt', '>', value]], orderBy: [['$createdAt', 'asc']] });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });

          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if $updatedAt timestamp used with ${type} value`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentH', { where: [['$updatedAt', '>', value]], orderBy: [['$updatedAt', 'asc']] });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });

          it('should return valid result if condition contains "$createdAt" field', async () => {
            const result = await documentRepository.find(dataContract, 'documentI', { where: [['$createdAt', '==', Date.now()]] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return valid result if condition contains "$updatedAt" field', async () => {
            const result = await documentRepository.find(dataContract, 'documentH', { where: [['$updatedAt', '==', Date.now()]] });

            expect(result).to.be.instanceOf(StorageResult);
          });
        });

        describe('in', () => {
          it('should return valid result if "in" operator used with an array value', async () => {
            const result = await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', [1, 2]]], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          notArrayTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "in" operator used with not an array value, but ${type}`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', value]], orderBy: [['a', 'asc']] });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal(':Invalid query: invalid IN clause error: when using in operator you must provide an array of values');
              }
            });
          });

          it('should return invalid result if "in" operator used with an empty array value', async () => {
            try {
              await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', []]], orderBy: [['a', 'asc']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "in" operator used with an array value which contains more than 100 elements', async () => {
            const arr = [];

            for (let i = 0; i < 100; i++) {
              arr.push(i);
            }

            const result = await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);

            arr.push(101);

            try {
              documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', arr]] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "in" operator used with an array which contains not unique elements', async () => {
            const arr = [1, 1];
            try {
              await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
            }
          });

          it('should return invalid results if condition contains empty arrays', async () => {
            const arr = [[], []];
            try {
              await documentRepository.find(dataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
            }
          });
        });

        describe('startsWith', () => {
          it('should return valid result if "startsWith" operator used with a string value', async () => {
            const result = await documentRepository.find(dataContract, 'documentString', { where: [['a', 'startsWith', 'b']], orderBy: [['a', 'asc']] });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return invalid result if "startsWith" operator used with an empty string value', async () => {
            try {
              await documentRepository.find(dataContract, 'documentString', { where: [['a', 'startsWith', '']], orderBy: [['a', 'asc']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "startsWith" operator used with a string value which is more than 255 chars long', async () => {
            const value = 'b'.repeat(256);
            try {
              await documentRepository.find(dataContract, 'documentString', { where: [['a', 'startsWith', value]], orderBy: [['a', 'asc']] });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          nonStringTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "startWith" operator used with a not string value, but ${type}`, async () => {
              try {
                await documentRepository.find(dataContract, 'documentString', { where: [['a', 'startsWith', value]], orderBy: [['a', 'asc']] });
                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });
        });

        describe.skip('elementMatch', () => {
          it('should return valid result if "elementMatch" operator used with "where" conditions', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1], ['elem', '<', 3]],
                ],
              ],
            };

            const result = await documentRepository.find(dataContract, 'document', query);

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return invalid result if "elementMatch" operator used with invalid "where" conditions', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['elem', 'startsWith', 1], ['elem', '<', 3]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "elementMatch" operator used with less than 2 "where" conditions', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if value contains conflicting conditions', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['elem', '>', 1], ['elem', '>', 1]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if $id field is specified', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['$id', '>', 1], ['$id', '<', 3]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if $ownerId field is specified', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['$ownerId', '>', 1], ['$ownerId', '<', 3]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if value contains nested "elementMatch" operator', async () => {
            const query = {
              where: [
                ['arr', 'elementMatch',
                  [['subArr', 'elementMatch', [
                    ['subArrElem', '>', 1], ['subArrElem', '<', 3],
                  ]], ['subArr', '<', 3]],
                ],
              ],
            };

            try {
              await documentRepository.find(dataContract, 'document', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });
        });

        describe.skip('length', () => {
          it('should return valid result if "length" operator used with a positive numeric value', async () => {
            const result = await documentRepository.find(dataContract, 'document', {
              where: [
                ['arr', 'length', 2],
              ],
            });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return valid result if "length" operator used with zero', async () => {
            const result = await documentRepository.find(dataContract, 'document', {
              where: [
                ['arr', 'length', 0],
              ],
            });

            expect(result).to.be.instanceOf(StorageResult);
          });

          it('should return invalid result if "length" operator used with a float numeric value', async () => {
            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'length', 1.2],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "length" operator used with a NaN', async () => {
            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'length', NaN],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "length" operator used with a numeric value which is less than 0', async () => {
            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'length', -1],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          nonNumberTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if "length" operator used with a ${type} instead of numeric value`, async () => {
              try {
                await documentRepository.find(dataContract, 'document', {
                  where: [
                    ['arr', 'length', value],
                  ],
                });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });
        });

        describe.skip('contains', () => {
          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "contains" operator used with a scalar value ${type}`, async () => {
              const result = await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'contains', value],
                ],
              });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          scalarTestCases.forEach(({ type, value }) => {
            it(`should return valid result if "contains" operator used with an array of scalar values ${type}`, async () => {
              const result = await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'contains', [value]],
                ],
              });

              expect(result).to.be.instanceOf(StorageResult);
            });
          });

          it('should return invalid result if "contains" operator used with an array which has '
            + ' more than 100 elements', async () => {
            const arr = [];
            for (let i = 0; i < 100; i++) {
              arr.push(i);
            }

            const result = await documentRepository.find(dataContract, 'document', {
              where: [
                ['arr', 'contains', arr],
              ],
            });

            expect(result).to.be.instanceOf(StorageResult);

            arr.push(101);

            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'contains', arr],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "contains" operator used with an empty array', async () => {
            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'contains', []],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          it('should return invalid result if "contains" operator used with an array which contains not unique'
            + ' elements', async () => {
            try {
              await documentRepository.find(dataContract, 'document', {
                where: [
                  ['arr', 'contains', [1, 1]],
                ],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('');
            }
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if used with non-scalar value ${type}`, async () => {
              try {
                await documentRepository.find(dataContract, 'document', {
                  where: [
                    ['arr', 'contains', value],
                  ],
                });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });

          nonScalarTestCases.forEach(({ type, value }) => {
            it(`should return invalid result if used with an array of non-scalar values ${type}`, async () => {
              try {
                await documentRepository.find(dataContract, 'document', {
                  where: [
                    ['arr', 'contains', [value]],

                  ],
                });

                expect.fail('should throw an error');
              } catch (e) {
                expect(e).to.be.instanceOf(InvalidQueryError);
                expect(e.message).to.equal('');
              }
            });
          });
        });
      });
    });
  });

  describe('limit', () => {
    it('should return valid result if "limit" is a number', async () => {
      const result = await documentRepository.find(dataContract, 'documentNumber', {
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc']],
        limit: 1,
      });

      expect(result).to.be.instanceOf(StorageResult);
    });

    it('should return invalid result if "limit" is less than 1', async () => {
      const where = [
        ['a', '>', 1],
      ];

      try {
        await documentRepository.find(dataContract, 'documentNumber', { where, limit: 0, orderBy: [['a', 'asc']] });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }

      try {
        await documentRepository.find(dataContract, 'documentNumber', { where, limit: -1, orderBy: [['a', 'asc']] });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }
    });

    it('should return invalid result if "limit" is bigger than 100', async () => {
      const where = [
        ['a', '>', 1],
      ];

      const result = await documentRepository.find(dataContract, 'documentNumber', { where, limit: 100, orderBy: [['a', 'asc']] });

      expect(result).to.be.instanceOf(StorageResult);

      try {
        await documentRepository.find(dataContract, 'documentNumber', { where, limit: 101, orderBy: [['a', 'asc']] });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }
    });

    it('should return invalid result if "limit" is a float number', async () => {
      const where = [
        ['a', '>', 1],
      ];

      try {
        await documentRepository.find(dataContract, 'documentNumber', { where, limit: 1.5, orderBy: [['a', 'asc']] });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: query invalid limit error: limit should be a integer from 1 to 100');
      }
    });

    nonNumberAndUndefinedTestCases.forEach(({ type, value }) => {
      it(`should return invalid result if "limit" is not a number, but ${type}`, async () => {
        try {
          await documentRepository.find(dataContract, 'documentNumber', {
            where: [
              ['a', '>', 1],
            ],
            limit: value,
            orderBy: [['a', 'asc']],
          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          expect(e.message).to.equal('Invalid query: query invalid limit error: limit should be a integer from 1 to 100');
        }
      });
    });
  });

  describe('orderBy', () => {
    it('should return valid result if "orderBy" contains 1 sorting field', async () => {
      const result = await documentRepository.find(dataContract, 'documentNumber', {
        where: [
          ['a', '>', 1],
        ],
        orderBy: [['a', 'asc']],
      });

      expect(result).to.be.instanceOf(StorageResult);
    });

    it('should return invalid result if "orderBy" contains 2 sorting fields', async () => {
      try {
        await documentRepository.find(dataContract, 'documentC', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [['a', 'asc'], ['b', 'desc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }
    });

    it('should return invalid result if "orderBy" is an empty array', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
      }
    });

    it('should return invalid result if sorting applied to not range condition', async () => {
      try {
        await documentRepository.find(dataContract, 'documentString', {
          where: [['a', '==', 'b']],
          orderBy: [['a', 'asc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }
    });

    it('should return invalid result if there is no where conditions', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          orderBy: [['a', 'asc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }
    });

    it('should return invalid result if the field inside an "orderBy" is an empty array', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [[]],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
      }
    });

    it('should return invalid result if "orderBy" has more than 255 sorting fields', async () => {
      try {
        await documentRepository.find(dataContract, 'documentBig', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: Array(256).fill().map((v, i) => [`a${i}`, 'asc']),
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        // TODO is it correct?
        expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
      }
    });

    it('should return invalid result if order of three of two properties after indexed one is not preserved', async () => {
      // documentSchema = {
      //   indices: [
      //     {
      //       name: 'index1',
      //       properties: [
      //         { a: 'asc' },
      //         { b: 'desc' },
      //         { c: 'desc' },
      //         { d: 'desc' },
      //         { e: 'desc' },
      //       ],
      //       unique: true,
      //     },
      //   ],
      // };
      //
      // findThreesomeOfIndexedPropertiesStub.returns([['b', 'c', 'd']]);
      // findIndexedPropertiesSinceStub.returns([['b', 'c']]);
      // findAppropriateIndexStub.returns({
      //   properties: ['b', 'c'],
      // });
      //

      try {
        await documentRepository.find(dataContract, 'documentL', {
          where: [
            ['b', '>', 1],
          ],
          orderBy: [['b', 'desc'], ['e', 'asc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
      }
    });

    it('should return invalid result if order of properties does not match index', async () => {
      // documentSchema = {
      //   indices: [
      //     {
      //       name: 'index1',
      //       properties: [
      //         { a: 'asc' },
      //         { b: 'desc' },
      //         { c: 'desc' },
      //         { d: 'desc' },
      //         { e: 'desc' },
      //       ],
      //       unique: true,
      //     },
      //   ],
      // };
      //
      // findThreesomeOfIndexedPropertiesStub.returns([['b', 'c', 'd']]);
      // findIndexedPropertiesSinceStub.returns([['b', 'c']]);
      // findAppropriateIndexStub.returns({
      //   properties: ['b', 'c'],
      // });

      try {
        await documentRepository.find(dataContract, 'documentJ', {
          where: [
            ['b', '>', 1],
          ],
          orderBy: [['b', 'desc'], ['d', 'asc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
      }
    });

    validFieldNameTestCases.forEach((fieldName) => {
      it(`should return valid result if "orderBy" has valid field format, ${fieldName}`, async () => {
        const result = await documentRepository.find(dataContract, `document${fieldName}`, {
          where: [
            [fieldName, '>', fieldName.startsWith('$') && !fieldName.endsWith('At') ? generateRandomIdentifier() : 1],
          ],
          orderBy: [[fieldName, 'asc']],
        });

        expect(result).to.be.instanceOf(StorageResult);
      });
    });

    invalidFieldNameTestCases.forEach((fieldName) => {
      it(`should return invalid result if "orderBy" has invalid field format, ${fieldName}`, async () => {
        // documentSchema = {
        //   indices: [
        //     {
        //       properties: [{ [fieldName]: 'asc' }],
        //     },
        //   ],
        // };
        //

        try {
          await documentRepository.find(dataContract, 'document', {
            where: [
              ['a', '>', 1],
            ],
            orderBy: [['$a', 'asc']],
          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          expect(e.message).to.equal('');
        }
      });
    });

    it('should return invalid result if "orderBy" has wrong direction', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [['a', 'a']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
      }
    });

    it('should return invalid result if "orderBy" field array has less than 2 elements (field, direction)', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [['a']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
      }
    });

    it('should return invalid result if "orderBy" field array has more than 2 elements (field, direction)', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', '>', 1],
          ],
          orderBy: [['a', 'asc', 'desc']],
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: missing order by for range error: query must have an orderBy field for each range element');
      }
    });

    Object.keys(validOrderByOperators).forEach((operator) => {
      it(`should return valid result if "orderBy" has valid field with valid operator in "where" clause - "${operator}"`, async () => {
        const result = await documentRepository.find(dataContract, 'documentNumber', {
          where: [
            ['a', operator, validOrderByOperators[operator].value],
          ],
          orderBy: [['a', 'asc']],
        });

        expect(result).to.be.instanceOf(StorageResult);
      });
    });

    it('should return invalid result if "orderBy" was not used with range operator', async () => {
      const query = {
        orderBy: [['b', 'asc']],
      };

      try {
        await documentRepository.find(dataContract, 'documentK', query);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }

      query.where = [['a', '==', 1]];

      try {
        await documentRepository.find(dataContract, 'documentK', query);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('');
      }

      const promises = ['>', '<', '>=', '<=', 'startsWith', 'in'].map(async (operator) => {
        let value = '1';
        if (operator === 'in') {
          value = [1];
        }

        query.where = [['b', operator, value]];

        const result = await documentRepository.find(dataContract, 'documentK', query);

        expect(result).to.be.instanceOf(StorageResult);
      });

      await Promise.all(promises);
    });
  });

  describe('startAt', () => {
    let id;

    before(async () => {
      const documentFactory = new DocumentFactory(
        createDPPMock(),
        () => ({
          isValid: () => true,
        }),
        () => {},
      );

      const ownerId = generateRandomIdentifier();
      const doc = documentFactory.create(dataContract, ownerId, 'documentNumber', { a: 2 });
      await documentRepository.store(doc);

      const storedDoc = await documentRepository.find(dataContract, 'documentNumber', {
        where: [[
          'a', '==', 2,
        ]],
      });

      id = storedDoc.getValue()[0].getId();
    });

    [...nonNumberAndUndefinedTestCases, typesTestCases.number].forEach(({ type, value }) => {
      it(`should return invalid result if "startAt" is not a number, but ${type}`, async function it() {
        if (type === 'buffer') {
          this.skip();
        }

        try {
          await documentRepository.find(dataContract, 'documentNumber', {
            startAt: value,
          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          // TODO wrong error ?
          expect(e.message).to.equal('');
        }
      });
    });

    it('should return valid result if "startAt" is an Identifier', async () => {
      const result = await documentRepository.find(dataContract, 'documentNumber', {
        startAt: id,
      });

      expect(result).to.be.instanceOf(StorageResult);
    });
  });

  describe('startAfter', () => {
    let id;

    before(async () => {
      const documentFactory = new DocumentFactory(
        createDPPMock(),
        () => ({
          isValid: () => true,
        }),
        () => {},
      );

      const ownerId = generateRandomIdentifier();
      const doc = documentFactory.create(dataContract, ownerId, 'documentNumber', { a: 1 });
      await documentRepository.store(doc);

      const storedDoc = await documentRepository.find(dataContract, 'documentNumber', {
        where: [[
          'a', '==', 1,
        ]],
      });

      id = storedDoc.getValue()[0].getId();
    });

    it('should return invalid result if both "startAt" and "startAfter" are present', async () => {
      try {
        await documentRepository.find(dataContract, 'documentNumber', {
          startAfter: id,
          startAt: id,
        });

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(InvalidQueryError);
        expect(e.message).to.equal('Invalid query: duplicate start conditions error: only one of startAt or startAfter should be provided');
      }
    });

    [...nonNumberAndUndefinedTestCases, typesTestCases.number].forEach(({ type, value }) => {
      it(`should return invalid result if "startAfter" is not a number, but ${type}`, async function it() {
        if (type === 'buffer') {
          this.skip();
        }

        try {
          await documentRepository.find(dataContract, 'documentNumber', {
            startAfter: value,
          });

          expect.fail('should throw an error');
        } catch (e) {
          expect(e).to.be.instanceOf(InvalidQueryError);
          // TODO wrong error ?
          expect(e.message).to.equal('');
        }
      });
    });

    it('should return valid result if "startAfter" is an Identifier', async () => {
      const result = await documentRepository.find(dataContract, 'documentNumber', {
        startAfter: id,
      });

      expect(result).to.be.instanceOf(StorageResult);
    });
  });
});
