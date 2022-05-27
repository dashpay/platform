const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const DataContractFactory = require('@dashevo/dpp/lib/dataContract/DataContractFactory');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const createDocumentTypeTreePath = require('../../../lib/document/groveDB/createDocumentTreePath');
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

const queryDocumentSchema = {
  testDocument: {
    type: 'object',
    properties: {
      firstName: {
        type: 'string',
      },
      lastName: {
        type: 'string',
      },
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
    required: ['$createdAt'],
    additionalProperties: false,
    indices: [
      {
        name: 'one',
        properties: [
          { firstName: 'asc' },
        ],
      },
      {
        name: 'two',
        properties: [
          { a: 'asc' },
          { b: 'asc' },
          { c: 'asc' },
          { d: 'asc' },
          { e: 'asc' },
        ],
      },
    ],
  },
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
  queryDocumentSchema[`document${fieldName}`] = {
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

  queryDocumentSchema[`document${ucFirst(type)}`] = {
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

queryDocumentSchema.documentBig = {
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

const validQueries = [
  {},
  {
    where: [['$id', 'in', [
      generateRandomIdentifier(),
      generateRandomIdentifier(),
      generateRandomIdentifier(),
    ]]],
    orderBy: [['$id', 'asc']],
  },
  {
    where: [
      ['a', '==', 1],
      ['b', '==', 2],
      ['c', '==', 3],
      ['d', 'in', [1, 2]],
    ],
    orderBy: [
      ['d', 'desc'],
      ['e', 'asc'],
    ],
  },
  {
    where: [
      ['a', '==', 1],
      ['b', '==', 2],
      ['c', '==', 3],
      ['d', 'in', [1, 2]],
      ['e', '>', 3],
    ],
    orderBy: [
      ['d', 'desc'],
      ['e', 'asc'],
    ],
  },
  {
    where: [
      ['firstName', '>', 'Chris'],
      ['firstName', '<=', 'Noellyn'],
    ],
    orderBy: [
      ['firstName', 'asc'],
    ],
  },
];

const invalidQueries = [
  {
    query: {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
      ],
    },
    error: 'Invalid query: query is too far from index: query must better match an existing index',
  },
  {
    query: {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['c', 'in', [1, 2]],
      ],
      orderBy: [
        ['c', 'desc'],
      ],
    },
    error: 'Invalid query: where clause on non indexed property error: query must be for valid indexes',
  },
  {
    query: {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['b', 'in', [1, 2]],
      ],
      orderBy: [
        ['b', 'desc'],
      ],
    },
    error: 'Invalid query: duplicate non groupable clause on same field error: in clause has same field as an equality clause',
  },
  {
    query: {
      where: [
        ['z', '==', 1],
      ],
    },
    error: 'Invalid query: where clause on non indexed property error: query must be for valid indexes',
  },
  {
    query: {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['c', '>', 3],
        ['d', 'in', [1, 2]],
        ['e', '>', 3],
      ],
    },
    error: 'Invalid query: multiple range clauses error: all ranges must be on same field',
  },
  {
    query: {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['c', '>', 3],
        ['d', '>', 3],
      ],
      orderBy: [
        ['c', 'asc'],
        ['d', 'desc'],
      ],
    },
    error: 'Invalid query: multiple range clauses error: all ranges must be on same field',
  },
  {
    query: {
      where: [
        ['a', '==', 3],
        ['b', '==', 2],
        ['c', '>', 1],
      ],
    },
    error: 'Invalid query: missing order by for range error: query must have an orderBy field for each range element',
  },
  {
    query: {
      where: [
        ['a', '==', 3],
        ['b', '==', 2],
        ['c', '==', 3],
        ['d', 'in', [1, 2]],
        ['e', '<', 1],
      ],
      orderBy: [
        ['e', 'asc'],
        ['d', 'asc'],
      ],
    },
    error: 'Invalid query: where clause on non indexed property error: query must be for valid indexes',
  },
  {
    query: 'abc',
    error: 'Invalid query: invalid cbor error: unable to decode query',
  },
  {
    query: [],
    error: 'Invalid query: invalid cbor error: unable to decode query',
  },
  {
    query: { where: [1, 2, 3] },
    error: 'Invalid query: query invalid format for where clause error: where clause must be an array',
  },
  {
    query: { invalid: 'query' },
    error: '',
  },
];

const invalidOperators = ['<<', '<==', '===', '!>', '>>='];

async function createDocuments(documentRepository, documents) {
  return Promise.all(
    documents.map(async (o) => {
      const result = await documentRepository.store(o);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);
    }),
  );
}

describe('DocumentRepository', function main() {
  this.timeout(30000);

  let documentRepository;
  let dataContractRepository;
  let container;
  let dataContract;
  let queryDataContract;
  let documents;
  let document;
  let documentSchema;

  beforeEach(async () => {
    container = await createTestDIContainer();

    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract).slice(0, 5);

    [document] = documents;

    // Modify documents for the test cases
    documents = documents.map((doc, i) => {
      const currentDocument = doc;
      // const arrayItem = { item: i + 1, flag: true };

      currentDocument.set('order', i);
      // currentDocument.set('arrayWithScalar', Array(i + 1)
      //   .fill(1)
      //   .map((item, index) => i + index));
      // currentDocument.set('arrayWithObjects', Array(i + 1).fill(arrayItem));
      currentDocument.type = document.getType();

      return currentDocument;
    });

    [document] = documents;

    dataContract.documents[document.getType()].properties = {
      ...dataContract.documents[document.getType()].properties,
      // name: {
      //   type: 'string',
      //   // maxLength: 1024,
      // },
      order: {
        type: 'number',
      },
      lastName: {
        type: 'string',
        // maxLength: 1024,
      },
      // arrayWithScalar: {
      //   type: 'array',
      //   items: [
      //     { type: 'string' },
      //   ],
      // },
      // arrayWithObjects: {
      //   type: 'array',
      //   items: {
      //     type: 'object',
      //     properties: {
      //       flag: {
      //         type: 'string',
      //       },
      //     },
      //   },
      // },
    };
    //
    const documentsSchema = dataContract.getDocuments();

    documentSchema = documentsSchema[document.getType()];

    // redeclare indices
    const indices = documentSchema.indices || [];
    documentSchema.indices = indices.concat([
      {
        name: 'index1',
        properties: [{ name: 'asc' }],
      },
      // {
      //   name: 'index2',
      //
      //   properties: [{ name: 'asc' }, { 'arrayWithObjects.item': 'asc' }],
      // },
      {
        name: 'index3',
        properties: [{ order: 'asc' }],
      },
      {
        name: 'index4',
        properties: [{ lastName: 'asc' }],
      },
      // {
      //   name: 'index5',
      //   properties: [{ arrayWithScalar: 'asc' }],
      // },
      // {
      //   name: 'index6',
      //   properties: [{ arrayWithObjects: 'asc' }],
      // },
      // {
      //   name: 'index7',
      //   properties: [{ 'arrayWithObjects.item': 'asc' }],
      // },
      // {
      //   name: 'index8',
      //   properties: [{ 'arrayWithObjects.flag': 'asc' }],
      // },
      // {
      //   name: 'index9',
      //   properties: [{ primaryOrder: 'asc' }, { order: 'desc' }],
      // },
      {
        name: 'index10',
        properties: [{ $ownerId: 'desc' }],
      },
    ]);

    const dpp = container.resolve('dpp');
    queryDataContract = dpp.dataContract.create(generateRandomIdentifier(), queryDocumentSchema);

    documentRepository = container.resolve('documentRepository');

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    dataContractRepository = container.resolve('dataContractRepository');

    await dataContractRepository.store(dataContract);
    await dataContractRepository.store(queryDataContract);
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  describe('#store', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should store Document', async () => {
      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      const result = await documentRepository
        .storage
        .db
        .get(documentTreePath, document.getId().toBuffer(), false);

      expect(document.toBuffer()).to.deep.equal(result.value);
    });

    it('should store Document in transaction', async () => {
      await documentRepository.delete(dataContract, document.getType(), document.getId());

      await documentRepository
        .storage
        .startTransaction();

      await documentRepository.store(document, true);

      const documentTypeTreePath = createDocumentTypeTreePath(
        document.getDataContract(),
        document.getType(),
      );

      const documentTreePath = documentTypeTreePath.concat(
        [Buffer.from([0])],
      );

      const transactionDocument = await documentRepository
        .storage
        .db
        .get(documentTreePath, document.getId().toBuffer(), true);

      try {
        await documentRepository
          .storage
          .db
          .get(documentTreePath, document.getId().toBuffer(), false);

        expect.fail('should fail with NotFoundError error');
      } catch (e) {
        expect(e.message.startsWith('path key not found: key not found in Merk')).to.be.true();
      }

      await documentRepository.storage.commitTransaction();

      const createdDocument = await documentRepository
        .storage
        .db
        .get(documentTreePath, document.getId().toBuffer(), false);

      expect(document.toBuffer()).to.deep.equal(transactionDocument.value);
      expect(document.toBuffer()).to.deep.equal(createdDocument.value);
    });
  });

  describe('#find', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should find all existing documents', async () => {
      const result = await documentRepository.find(dataContract, document.getType());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const foundDocuments = result.getValue();

      expect(foundDocuments).to.be.an('array');
      expect(foundDocuments).to.have.lengthOf(documents.length);

      const foundDocumentsBuffers = foundDocuments.map((doc) => doc.toBuffer());

      expect(foundDocumentsBuffers).to.have.deep.members(documents.map((doc) => doc.toBuffer()));
    });

    it('should find all existing documents in transaction', async () => {
      await documentRepository
        .storage
        .startTransaction();

      const foundDocumentsResult = await documentRepository
        .find(dataContract, document.getType(), {}, true);

      expect(foundDocumentsResult).to.be.instanceOf(StorageResult);
      expect(foundDocumentsResult.getOperations().length).to.be.greaterThan(0);

      const foundDocuments = foundDocumentsResult.getValue();

      await documentRepository.storage.commitTransaction();

      expect(foundDocuments).to.be.an('array');
      expect(foundDocuments).to.have.lengthOf(documents.length);

      const foundDocumentsBuffers = foundDocuments.map((doc) => doc.toBuffer());

      expect(foundDocumentsBuffers).to.have.deep.members(documents.map((doc) => doc.toBuffer()));
    });

    describe('queries', () => {
      describe('valid queries', () => {
        validQueries.forEach((query) => {
          it(`should return valid result for query "${JSON.stringify(query)}"`, async () => {
            const result = await documentRepository.find(queryDataContract, 'testDocument', query);

            expect(result).to.be.instanceOf(StorageResult);
          });
        });

        it('should return valid result if data contract has only system properties', async () => {
          const schema = {
            chat: {
              type: 'object',
              indices: [
                {
                  name: 'createdAt',
                  properties: [
                    {
                      $createdAt: 'asc',
                    },
                  ],
                },
                {
                  name: '$ownerId',
                  properties: [
                    {
                      $ownerId: 'asc',
                    },
                  ],
                },
              ],
              properties: {
                type: 'object',
              },
              required: ['$createdAt'],
              additionalProperties: false,
            },
          };

          const factory = new DataContractFactory(createDPPMock(), () => {});
          const ownerId = generateRandomIdentifier();
          const myDataContract = factory.create(ownerId, schema);
          await dataContractRepository.store(myDataContract);

          const result = await documentRepository.find(myDataContract, 'chat', {
            where: [
              ['$ownerId', '==', ownerId],
              ['$createdAt', '>', new Date().getTime()],
            ],
            orderBy: [['$createdAt', 'asc']],
          });

          expect(result).to.be.instanceOf(StorageResult);
        });

        it('should return valid result for DPNS contract', async () => {
          const schema = {
            label: {
              type: 'object',
              properties: {
                normalizedLabel: {
                  type: 'string',
                },
                normalizedParentDomainName: {
                  type: 'string',
                },
              },
              indices: [
                {
                  name: 'index1',
                  properties: [
                    {
                      normalizedParentDomainName: 'asc',
                    },
                    {
                      normalizedLabel: 'asc',
                    },
                  ],
                  unique: true,
                },
              ],
            },
          };

          const factory = new DataContractFactory(createDPPMock(), () => {});
          const ownerId = generateRandomIdentifier();
          const myDataContract = factory.create(ownerId, schema);
          await dataContractRepository.store(myDataContract);

          const result = await documentRepository.find(myDataContract, 'label', {
            where: [
              ['normalizedParentDomainName', '==', 'dash'],
            ],
            orderBy: [['normalizedLabel', 'asc']],
          });

          expect(result).to.be.instanceOf(StorageResult);
        });
      });

      describe('invalid queries', () => {
          invalidQueries.forEach(({ query, error}) => {
          it(`should return throw InvalidQueryError for query "${JSON.stringify(query)}"`, async () => {
            try {
              await documentRepository.find(queryDataContract, 'testDocument', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal(error);
            }
          });
        });

        notObjectTestCases.forEach(({ type, value: query }) => {
          it(`should return invalid result if query is a ${type}`, async () => {
            try {
              await documentRepository.find(queryDataContract, 'documentA', query);

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: invalid cbor error: unable to decode query');
            }
          });
        });
      });

      describe('where', () => {
        it('should return empty array if where clause conditions do not match', async () => {
          const query = {
            where: [['name', '==', 'Dash enthusiast']],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.have.lengthOf(0);
        });

        it.skip('should find documents by nested object fields', async () => {
          const query = {
            where: [
              ['arrayWithObjects.item', '==', 2],
            ],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.an('array');
          expect(result).to.be.lengthOf(1);

          const [expectedDocument] = result;

          expect(expectedDocument.toBuffer()).to.deep.equal(documents[1].toBuffer());
        });

        it.skip('should return documents by several conditions', async () => {
          const query = {
            where: [
              ['name', '==', 'Cutie'],
              ['arrayWithObjects.item', '==', 1],
            ],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.an('array');
          expect(result).to.be.lengthOf(1);

          const [expectedDocument] = result;

          expect(expectedDocument.toBuffer()).to.deep.equal(documents[0].toBuffer());
        });

        notArrayTestCases.forEach(({ type, value: query }) => {
          it(`should return invalid result if "where" is not an array, but ${type}`, async () => {
            try {
              await documentRepository.find(queryDataContract, 'documentA', { where: query });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: query is too far from index: query must better match an existing index');
            }
          });
        });

        it('should return invalid result if "where" is an empty array', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentA', { where: [] });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if "where" contains more than 10 conditions', async () => {
          const where = Array(11).fill(['a', '<', 1]);
          try {
            await documentRepository.find(queryDataContract, 'documentA', { where });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: multiple range clauses error: there can only be at most 2 range clauses that must be on the same field');
          }
        });

        it('should return invalid result if "where" contains conflicting conditions', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentL', {
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
              const result = await documentRepository.find(queryDataContract, 'documentB', {
                where:
                  [['$id', '==', generateRandomIdentifier()]],
              });

              expect(result).to.be.instanceOf(StorageResult);
              expect(result.isEmpty()).to.be.true();
            });

            it('should return valid result if condition contains top-level field', async () => {
              const result = await documentRepository.find(queryDataContract, 'documentE', {
                where: [
                  ['a', '==', '1'],
                ],
              });

              expect(result).to.be.instanceOf(StorageResult);
              expect(result.isEmpty()).to.be.true();
            });

            it.skip('should return valid result if condition contains nested path field', async () => {
              const result = await documentRepository.find(queryDataContract, 'documentD', {
                where:
                  [['a.b', '==', '1']],
              });

              expect(result).to.be.instanceOf(StorageResult);
            });

            it('should return invalid result if property is not specified in document indices', async () => {
              try {
                await documentRepository.find(queryDataContract, 'documentD', {
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
          });

          it('should return invalid result if condition array has less than 3 elements (field, operator, value)', async () => {
            try {
              await documentRepository.find(queryDataContract, 'documentA', {
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
              await documentRepository.find(queryDataContract, 'documentA', {
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
              invalidOperators.forEach((operator) => {
                it('should return invalid result if condition contains invalid comparison operator', async () => {
                  const query = { where: [['a', operator, '1']] };
                  if (operator !== '===') {
                    query.orderBy = [['a', 'asc']];
                  }

                  try {
                    await documentRepository.find(queryDataContract, 'documentE', query);

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    expect(e.message).to.equal('Invalid query: invalid where clause components error: second field of where component should be a known operator');
                  }
                });
              });

              describe('<', () => {
                it('should find documents with "<" operator', async () => {
                  const query = {
                    where: [['order', '<', documents[1].get('order')]],
                    orderBy: [['order', 'asc']],
                  };

                  const result = await documentRepository.find(
                    dataContract,
                    document.getType(),
                    query,
                  );

                  expect(result).to.be.instanceOf(StorageResult);
                  expect(result.getOperations().length).to.be.greaterThan(0);

                  const foundDocuments = result.getValue();

                  expect(foundDocuments).to.be.an('array');
                  expect(foundDocuments).to.be.lengthOf(1);

                  const [expectedDocument] = foundDocuments;

                  expect(expectedDocument.toBuffer()).to.deep.equal(documents[0].toBuffer());
                });

                it('should return invalid result if "<" operator used with a string value longer than 64 chars', async () => {
                  const longString = 't'.repeat(64);

                  try {
                    await documentRepository.find(queryDataContract, 'documentString', { where: [['a', '<', longString]], orderBy: [['a', 'asc']] });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                  }

                  const veryLongString = 't'.repeat(65);

                  try {
                    await documentRepository.find(queryDataContract, 'documentString', { where: [['a', '<', veryLongString]], orderBy: [['a', 'asc']] });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
                  }
                });

                nonScalarTestCases.forEach(({ type, value }) => {
                  it(`should return invalid result if "<" operator used with a not scalar value, but ${type}`, async () => {
                    try {
                      await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', '<', value]], orderBy: [['a', 'asc']] });

                      expect.fail('should throw an error');
                    } catch (e) {
                      expect(e).to.be.instanceOf(InvalidQueryError);
                      expect(e.message).to.equal('Invalid query: value wrong type error: document field type doesn\'t match document value');
                    }
                  });
                });

                scalarTestCases.forEach(({ type, value }) => {
                  it(`should return valid result if "<" operator used with a scalar value ${type}`, async () => {
                    const docType = `document${ucFirst(type)}`;

                    const result = await documentRepository.find(queryDataContract, docType, { where: [['a', '<', value]], orderBy: [['a', 'asc']] });

                    expect(result).to.be.instanceOf(StorageResult);
                  });
                });
              });

              describe('<=', () => {
                scalarTestCases.forEach(({ type, value }) => {
                  it(`should return valid result if "<=" operator used with a scalar value ${type}`, async () => {
                    const result = await documentRepository.find(queryDataContract, `document${ucFirst(type)}`, { where: [['a', '<=', value]], orderBy: [['a', 'asc']] });

                    expect(result).to.be.instanceOf(StorageResult);
                  });
                });

                it('should find Documents using "<=" operator', async () => {
                  const query = {
                    where: [['order', '<=', documents[1].get('order')]],
                    orderBy: [['order', 'asc']],
                  };

                  const result = await documentRepository.find(
                    dataContract,
                    document.getType(),
                    query,
                  );

                  expect(result).to.be.instanceOf(StorageResult);
                  expect(result.getOperations().length).to.be.greaterThan(0);

                  const foundDocuments = result.getValue();

                  expect(foundDocuments).to.be.an('array');
                  expect(foundDocuments).to.be.lengthOf(2);

                  const expectedDocuments = documents.slice(0, 2).map((doc) => doc.toBuffer());

                  expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(
                    expectedDocuments,
                  );
                });
              });

              describe('==', () => {
                it('should find existing documents using "==" operator', async () => {
                  const query = {
                    where: [['name', '==', document.get('name')]],
                  };

                  const result = await documentRepository.find(
                    dataContract,
                    document.getType(),
                    query,
                  );

                  expect(result).to.be.instanceOf(StorageResult);
                  expect(result.getOperations().length).to.be.greaterThan(0);

                  const foundDocuments = result.getValue();

                  expect(foundDocuments).to.be.an('array');
                  expect(foundDocuments).to.be.lengthOf(1);

                  const [expectedDocument] = foundDocuments;

                  expect(expectedDocument.toBuffer()).to.deep.equal(document.toBuffer());
                });

                scalarTestCases.forEach(({ type, value }) => {
                  it(`should return valid result if "==" operator used with a scalar value ${type}`, async () => {
                    const result = await documentRepository.find(queryDataContract, `document${ucFirst(type)}`, { where: [['a', '==', value]] });

                    expect(result).to.be.instanceOf(StorageResult);
                  });
                });
              });

              describe('=>', () => {
                it('should find existing documents using ">=" operator', async () => {
                  const query = {
                    where: [['order', '>=', documents[1].get('order')]],
                    orderBy: [['order', 'asc']],
                  };

                  const result = await documentRepository.find(
                    dataContract,
                    document.getType(),
                    query,
                  );

                  expect(result).to.be.instanceOf(StorageResult);
                  expect(result.getOperations().length).to.be.greaterThan(0);

                  const foundDocuments = result.getValue();

                  expect(foundDocuments).to.be.an('array');
                  expect(foundDocuments).to.be.lengthOf(documents.length - 1);

                  documents.shift();
                  const expectedDocuments = documents
                    .map((doc) => doc.toBuffer());

                  expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(
                    expectedDocuments,
                  );
                });

                scalarTestCases.forEach(({ type, value }) => {
                  it(`should return valid result if ">=" operator used with a scalar value ${type}`, async () => {
                    const result = await documentRepository.find(queryDataContract, `document${ucFirst(type)}`, { where: [['a', '>=', value]], orderBy: [['a', 'asc']] });

                    expect(result).to.be.instanceOf(StorageResult);
                  });
                });
              });

              describe('>', () => {
                it('should find existing documents using ">" operator', async () => {
                  const query = {
                    where: [['order', '>', documents[1].get('order')]],
                    orderBy: [['order', 'asc']],
                  };

                  const result = await documentRepository.find(
                    dataContract,
                    document.getType(),
                    query,
                  );

                  expect(result).to.be.instanceOf(StorageResult);
                  expect(result.getOperations().length).to.be.greaterThan(0);

                  const foundDocuments = result.getValue();

                  expect(foundDocuments).to.be.an('array');
                  expect(foundDocuments).to.be.lengthOf(documents.length - 2);

                  const expectedDocuments = documents
                    .splice(2, documents.length)
                    .map((doc) => doc.toBuffer());

                  expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(
                    expectedDocuments,
                  );
                });

                scalarTestCases.forEach(({ type, value }) => {
                  it(`should return valid result if ">" operator used with a scalar value ${type}`, async () => {
                    const result = await documentRepository.find(queryDataContract, `document${ucFirst(type)}`, { where: [['a', '>', value]], orderBy: [['a', 'asc']] });

                    expect(result).to.be.instanceOf(StorageResult);
                  });
                });
              });

              ['>', '<', '<=', '>='].forEach((operator) => {
                it(`should return invalid results if "${operator}" used not in the last 2 where conditions`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'documentNumber', {
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
                        await documentRepository.find(queryDataContract, 'documentE', query);

                        expect.fail('should throw an error');
                      } catch (e) {
                        expect(e).to.be.instanceOf(InvalidQueryError);
                        expect(e.message).to.equal('Invalid query: multiple range clauses error: all ranges must be on same field');
                      }
                    });

                    await Promise.all(promises);
                  });
                });

                it('should return invalid result if "in" operator is used before last two indexed conditions', async () => {
                  const query = { where: [['a', 'in', [1, 2]]] };

                  try {
                    await documentRepository.find(queryDataContract, 'documentF', query);

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
                      await documentRepository.find(queryDataContract, 'documentF', query);
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
                      await documentRepository.find(queryDataContract, 'documentG', query);
                      expect.fail('should throw an error');
                    } catch (e) {
                      expect(e).to.be.instanceOf(InvalidQueryError);
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
                    await documentRepository.find(queryDataContract, 'documentF', query);
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
                    await documentRepository.find(queryDataContract, 'documentI', { where: [['$createdAt', '>', value]], orderBy: [['$createdAt', 'asc']] });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    expect(e.message).to.equal('Invalid query: value wrong type error: document field type doesn\'t match document value');
                  }
                });
              });

              nonNumberTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if $updatedAt timestamp used with ${type} value`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'documentH', { where: [['$updatedAt', '>', value]], orderBy: [['$updatedAt', 'asc']] });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    expect(e.message).to.equal(
                      'Invalid query: value wrong type error: document field type doesn\'t match document value',
                    );
                  }
                });
              });

              it('should return valid result if condition contains "$createdAt" field', async () => {
                const result = await documentRepository.find(queryDataContract, 'documentI', { where: [['$createdAt', '==', Date.now()]] });

                expect(result).to.be.instanceOf(StorageResult);
              });

              it('should return valid result if condition contains "$updatedAt" field', async () => {
                const result = await documentRepository.find(queryDataContract, 'documentH', { where: [['$updatedAt', '==', Date.now()]] });

                expect(result).to.be.instanceOf(StorageResult);
              });
            });

            describe('in', () => {
              it('should return valid result if "in" operator used with an array value', async () => {
                const query = {
                  where: [
                    ['$id', 'in', [
                      documents[0].getId(),
                      documents[1].getId(),
                    ]],
                  ],
                  orderBy: [['$id', 'asc']],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(2);

                const expectedDocuments = documents.slice(0, 2).map((doc) => doc.toBuffer());

                expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(
                  expectedDocuments,
                );
              });

              notArrayTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if "in" operator used with not an array value, but ${type}`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', value]], orderBy: [['a', 'asc']] });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                  }
                });
              });

              it('should return invalid result if "in" operator used with an empty array value', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', []]], orderBy: [['a', 'asc']] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "in" operator used with an array value which contains more than 100 elements', async () => {
                const arr = [];

                for (let i = 0; i < 100; i++) {
                  arr.push(i);
                }

                const result = await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

                expect(result).to.be.instanceOf(StorageResult);

                arr.push(101);

                try {
                  documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', arr]] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "in" operator used with an array which contains not unique elements', async () => {
                const arr = [1, 1];
                try {
                  await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid results if condition contains empty arrays', async () => {
                const arr = [[], []];
                try {
                  await documentRepository.find(queryDataContract, 'documentNumber', { where: [['a', 'in', arr]], orderBy: [['a', 'asc']] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                  expect(e.message).to.equal('Invalid query: value wrong type error: document field type doesn\'t match document value');
                }
              });
            });

            describe('startsWith', () => {
              it('should return valid result if "startsWith" operator used with a string value', async () => {
                const query = {
                  where: [['lastName', 'startsWith', 'Swe']],
                  orderBy: [['lastName', 'asc']],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(1);

                const [expectedDocument] = foundDocuments;

                expect(expectedDocument.toBuffer()).to.deep.equal(documents[2].toBuffer());
              });

              it('should return invalid result if "startsWith" operator used with an empty string value', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'documentString', { where: [['a', 'startsWith', '']], orderBy: [['a', 'asc']] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "startsWith" operator used with a string value which is more than 255 chars long', async () => {
                const value = 'b'.repeat(256);
                try {
                  await documentRepository.find(queryDataContract, 'documentString', { where: [['a', 'startsWith', value]], orderBy: [['a', 'asc']] });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              nonStringTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if "startWith" operator used with a not string value, but ${type}`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'documentString', { where: [['a', 'startsWith', value]], orderBy: [['a', 'asc']] });
                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                    expect(e.message).to.equal('Invalid query: value wrong type error: document field type doesn\'t match document value');
                  }
                });
              });
            });

            describe.skip('elementMatch', () => {
              it('should return valid result if "elementMatch" operator used with "where" conditions', async () => {
                const query = {
                  where: [
                    ['arrayWithObjects', 'elementMatch', [
                      ['item', '==', 2], ['flag', '==', true],
                    ]],
                  ],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(1);

                const [expectedDocument] = foundDocuments;

                expect(expectedDocument.toBuffer()).to.deep.equal(documents[1].toBuffer());
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
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
                  await documentRepository.find(queryDataContract, 'document', query);

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });
            });

            describe.skip('length', () => {
              it('should return valid result if "length" operator used with a positive numeric value', async () => {
                const query = {
                  where: [['arrayWithObjects', 'length', 2]],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(1);

                const [expectedDocument] = foundDocuments;

                expect(expectedDocument.toBuffer()).to.deep.equal(documents[1].toBuffer());
              });

              it('should return valid result if "length" operator used with zero', async () => {
                const result = await documentRepository.find(queryDataContract, 'document', {
                  where: [
                    ['arr', 'length', 0],
                  ],
                });

                expect(result).to.be.instanceOf(StorageResult);
              });

              it('should return invalid result if "length" operator used with a float numeric value', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'length', 1.2],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "length" operator used with a NaN', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'length', NaN],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "length" operator used with a numeric value which is less than 0', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'length', -1],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              nonNumberTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if "length" operator used with a ${type} instead of numeric value`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'document', {
                      where: [
                        ['arr', 'length', value],
                      ],
                    });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                  }
                });
              });
            });

            describe.skip('contains', () => {
              it('should find Documents using "contains" operator and array value', async () => {
                const query = {
                  where: [
                    ['arrayWithScalar', 'contains', [2, 3]],
                  ],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(1);

                const [expectedDocument] = foundDocuments;

                expect(expectedDocument.toBuffer()).to.deep.equal(documents[2].toBuffer());
              });

              it('should find Documents using "contains" operator and scalar value', async () => {
                const query = {
                  where: [
                    ['arrayWithScalar', 'contains', 2],
                  ],
                };

                const result = await documentRepository.find(
                  dataContract,
                  document.getType(),
                  query,
                );

                expect(result).to.be.instanceOf(StorageResult);
                expect(result.getOperations().length).to.be.greaterThan(0);

                const foundDocuments = result.getValue();

                expect(foundDocuments).to.be.an('array');
                expect(foundDocuments).to.be.lengthOf(2);

                const expectedDocuments = documents.slice(1, 3).map((doc) => doc.toBuffer());

                expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(
                  expectedDocuments,
                );
              });

              scalarTestCases.forEach(({ type, value }) => {
                it(`should return valid result if "contains" operator used with a scalar value ${type}`, async () => {
                  const result = await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'contains', value],
                    ],
                  });

                  expect(result).to.be.instanceOf(StorageResult);
                });
              });

              scalarTestCases.forEach(({ type, value }) => {
                it(`should return valid result if "contains" operator used with an array of scalar values ${type}`, async () => {
                  const result = await documentRepository.find(queryDataContract, 'document', {
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

                const result = await documentRepository.find(queryDataContract, 'document', {
                  where: [
                    ['arr', 'contains', arr],
                  ],
                });

                expect(result).to.be.instanceOf(StorageResult);

                arr.push(101);

                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'contains', arr],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "contains" operator used with an empty array', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'contains', []],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              it('should return invalid result if "contains" operator used with an array which contains not unique'
                + ' elements', async () => {
                try {
                  await documentRepository.find(queryDataContract, 'document', {
                    where: [
                      ['arr', 'contains', [1, 1]],
                    ],
                  });

                  expect.fail('should throw an error');
                } catch (e) {
                  expect(e).to.be.instanceOf(InvalidQueryError);
                }
              });

              nonScalarTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if used with non-scalar value ${type}`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'document', {
                      where: [
                        ['arr', 'contains', value],
                      ],
                    });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                  }
                });
              });

              nonScalarTestCases.forEach(({ type, value }) => {
                it(`should return invalid result if used with an array of non-scalar values ${type}`, async () => {
                  try {
                    await documentRepository.find(queryDataContract, 'document', {
                      where: [
                        ['arr', 'contains', [value]],

                      ],
                    });

                    expect.fail('should throw an error');
                  } catch (e) {
                    expect(e).to.be.instanceOf(InvalidQueryError);
                  }
                });
              });
            });
          });
        });
      });

      describe('limit', () => {
        it('should limit return to 1 Document if limit is set', async () => {
          const options = {
            limit: 1,
          };

          const result = await documentRepository.find(dataContract, document.getType(), options);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');
          expect(foundDocuments).to.have.lengthOf(1);
        });

        it('should limit result to 100 Documents if limit is not set', async () => {
          // Store 101 document
          for (let i = 0; i < 101; i++) {
            const svDoc = document;

            svDoc.id = Identifier.from(Buffer.alloc(32, i + 1));
            await documentRepository.store(svDoc);
          }

          const result = await documentRepository.find(dataContract, document.getType());

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');
          expect(foundDocuments).to.have.lengthOf(100);
        });

        it('should return valid result if "limit" is a number', async () => {
          const result = await documentRepository.find(queryDataContract, 'documentNumber', {
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

          // try {
          //   await documentRepository.find(queryDataContract, 'documentNumber', {
          //    where,
          //    limit: 0,
          //    orderBy: [['a', 'asc']],
          //   });
          //
          //   expect.fail('should throw an error');
          // } catch (e) {
          //   expect(e).to.be.instanceOf(InvalidQueryError);
          // }

          try {
            await documentRepository.find(queryDataContract, 'documentNumber', { where, limit: -1, orderBy: [['a', 'asc']] });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if "limit" is bigger than 100', async () => {
          const where = [
            ['a', '>', 1],
          ];

          const result = await documentRepository.find(queryDataContract, 'documentNumber', { where, limit: 100, orderBy: [['a', 'asc']] });

          expect(result).to.be.instanceOf(StorageResult);

          try {
            await documentRepository.find(queryDataContract, 'documentNumber', { where, limit: 101, orderBy: [['a', 'asc']] });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if "limit" is a float number', async () => {
          const where = [
            ['a', '>', 1],
          ];

          try {
            await documentRepository.find(queryDataContract, 'documentNumber', { where, limit: 1.5, orderBy: [['a', 'asc']] });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: query invalid limit error: limit should be a integer from 1 to 100');
          }
        });

        nonNumberAndUndefinedTestCases.forEach(({ type, value }) => {
          it(`should return invalid result if "limit" is not a number, but ${type}`, async () => {
            try {
              await documentRepository.find(queryDataContract, 'documentNumber', {
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

      describe('startAt', () => {
        it('should return Documents from 2 document', async () => {
          const query = {
            where: [
              ['order', '>=', 0],
            ],
            orderBy: [
              ['order', 'asc'],
            ],
            startAt: documents[1].id,
          };

          const result = await documentRepository.find(
            queryDataContract,
            document.getType(),
            query,
          );

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');

          const expectedDocuments = documents.splice(1).map((doc) => doc.toBuffer());

          expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
        });

        it('should throw InvalidQuery if document not found', async () => {
          const options = {
            startAt: Buffer.alloc(0),
          };

          try {
            await documentRepository.find(dataContract, document.getType(), options);

            expect.fail('should throw InvalidQueryError');
          } catch (e) {
            expect(e).to.be.an.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: start document not found error: startAt document not found');
          }
        });

        [...nonNumberAndUndefinedTestCases, typesTestCases.number].forEach(({ type, value }) => {
          it(`should return invalid result if "startAt" is not a number, but ${type}`, async function it() {
            if (type === 'buffer') {
              this.skip();
            }

            try {
              await documentRepository.find(queryDataContract, 'documentNumber', {
                startAt: value,
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
            }
          });
        });
      });

      describe('startAfter', () => {
        it('should return Documents after 1 document', async () => {
          const options = {
            where: [
              ['order', '>=', 0],
            ],
            orderBy: [
              ['order', 'asc'],
            ],
            startAfter: documents[0].id,
          };

          const result = await documentRepository.find(dataContract, document.getType(), options);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');

          const expectedDocuments = documents.splice(1).map((doc) => doc.toBuffer());

          expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
        });

        it('should throw InvalidQuery if document not found', async () => {
          const options = {
            startAfter: Buffer.alloc(0),
          };

          try {
            await documentRepository.find(dataContract, document.getType(), options);

            expect.fail('should throw InvalidQueryError');
          } catch (e) {
            expect(e).to.be.an.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: start document not found error: startAfter document not found');
          }
        });

        it('should return invalid result if both "startAt" and "startAfter" are present', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
              startAfter: documents[1].getId(),
              startAt: documents[1].getId(),
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
              await documentRepository.find(queryDataContract, 'documentNumber', {
                startAfter: value,
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: value wrong type error: system value is incorrect type');
            }
          });
        });
      });

      describe('orderBy', () => {
        it('should sort Documents in descending order', async () => {
          const query = {
            where: [
              ['order', '>=', 0],
            ],
            orderBy: [
              ['order', 'desc'],
            ],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');

          const expectedDocuments = documents.reverse().map((doc) => doc.toBuffer());

          expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.equal(expectedDocuments);
        });

        it('should sort Documents in ascending order', async () => {
          const query = {
            where: [
              ['order', '>=', 0],
            ],
            orderBy: [
              ['order', 'asc'],
            ],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');

          const expectedDocuments = documents.map((doc) => doc.toBuffer());

          expect(foundDocuments.map((doc) => doc.toBuffer())).to.deep.equal(expectedDocuments);
        });

        it('should sort Documents by $id', async () => {
          await Promise.all(
            documents.map((d) => documentRepository
              .delete(dataContract, document.getType(), d.getId())),
          );

          const createdIds = [];
          let i = 0;
          for (const svDoc of documents) {
            svDoc.id = Identifier.from(Buffer.alloc(32, i + 1));
            await documentRepository.store(svDoc);
            i++;
            createdIds.push(svDoc.id);
          }

          const query = {
            where: [
              ['$id', 'in', createdIds],
            ],
            orderBy: [
              ['$id', 'desc'],
            ],
          };

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.instanceOf(StorageResult);
          expect(result.getOperations().length).to.be.greaterThan(0);

          const foundDocuments = result.getValue();

          expect(foundDocuments).to.be.an('array');
          expect(foundDocuments).to.be.lengthOf(documents.length);

          expect(foundDocuments[0].getId()).to.deep.equal(createdIds[4]);
          expect(foundDocuments[1].getId()).to.deep.equal(createdIds[3]);
          expect(foundDocuments[2].getId()).to.deep.equal(createdIds[2]);
          expect(foundDocuments[3].getId()).to.deep.equal(createdIds[1]);
          expect(foundDocuments[4].getId()).to.deep.equal(createdIds[0]);
        });

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
            await documentRepository.find(queryDataContract, 'documentC', {
              where: [
                ['a', '>', 1],
              ],
              orderBy: [['a', 'asc'], ['b', 'desc']],
            });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if "orderBy" is an empty array', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentString', {
              where: [['a', '==', 'b']],
              orderBy: [['a', 'asc']],
            });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if there is no where conditions', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
              orderBy: [['a', 'asc']],
            });

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }
        });

        it('should return invalid result if the field inside an "orderBy" is an empty array', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentBig', {
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
          // testDocumentSchema = {
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
            await documentRepository.find(queryDataContract, 'documentL', {
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
          // testDocumentSchema = {
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
            await documentRepository.find(queryDataContract, 'documentJ', {
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
            const result = await documentRepository.find(queryDataContract, `document${fieldName}`, {
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
              await documentRepository.find(queryDataContract, 'documentNumber', {
                where: [
                  ['a', '>', 1],
                ],
                orderBy: [['$a', 'asc']],
              });

              expect.fail('should throw an error');
            } catch (e) {
              expect(e).to.be.instanceOf(InvalidQueryError);
              expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
            }
          });
        });

        it('should return invalid result if "orderBy" has wrong direction', async () => {
          try {
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentNumber', {
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
            const result = await documentRepository.find(queryDataContract, 'documentNumber', {
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
            await documentRepository.find(queryDataContract, 'documentK', query);

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
          }

          query.where = [['a', '==', 1]];

          try {
            await documentRepository.find(queryDataContract, 'documentK', query);

            expect.fail('should throw an error');
          } catch (e) {
            expect(e).to.be.instanceOf(InvalidQueryError);
            expect(e.message).to.equal('Invalid query: where clause on non indexed property error: query must be for valid indexes');
          }

          const promises = ['>', '<', '>=', '<=', 'startsWith', 'in'].map(async (operator) => {
            let value = '1';
            if (operator === 'in') {
              value = [1];
            }

            query.where = [['b', operator, value]];

            const result = await documentRepository.find(queryDataContract, 'documentK', query);

            expect(result).to.be.instanceOf(StorageResult);
          });

          await Promise.all(promises);
        });
      });
    });
  });

  describe('#delete', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should delete Document', async () => {
      let result = await documentRepository.delete(
        dataContract,
        document.getType(),
        document.getId(),
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      result = await documentRepository.find(dataContract, document.getType(), {
        where: [['$id', '==', document.getId()]],
      });

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const foundDocuments = result.getValue();

      expect(foundDocuments).to.have.lengthOf(0);
    });

    it('should delete Document in transaction', async () => {
      await documentRepository
        .storage
        .startTransaction();

      const result = await documentRepository.delete(
        dataContract,
        document.getType(),
        document.getId(),
        true,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const removedDocumentResult = await documentRepository
        .find(
          dataContract,
          document.getType(),
          query,
          true,
        );

      const removedDocument = removedDocumentResult.getValue();

      const notRemovedDocumentsResult = await documentRepository
        .find(dataContract, document.getType(), query);

      const notRemovedDocuments = notRemovedDocumentsResult.getValue();

      await documentRepository
        .storage.commitTransaction();

      const completelyRemovedDocumentResult = await documentRepository
        .find(dataContract, document.getType(), query);

      const completelyRemovedDocument = completelyRemovedDocumentResult.getValue();

      expect(removedDocument).to.have.lengthOf(0);
      expect(notRemovedDocuments).to.be.not.null();
      expect(notRemovedDocuments[0].toBuffer()).to.deep.equal(document.toBuffer());
      expect(completelyRemovedDocument).to.have.lengthOf(0);
    });

    it('should restore document if transaction aborted', async () => {
      await documentRepository
        .storage
        .startTransaction();

      await documentRepository.delete(dataContract, document.getType(), document.getId(), true);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      // Document should be removed in transaction

      const removedDocumentsResult = await documentRepository
        .find(dataContract, document.getType(), query, true);

      const removedDocuments = removedDocumentsResult.getValue();

      expect(removedDocuments).to.have.lengthOf(0);

      // But still exists in main database

      const removedDocumentsWithoutTransactionResult = await documentRepository
        .find(dataContract, document.getType(), query);

      const removedDocumentsWithoutTransaction = removedDocumentsWithoutTransactionResult
        .getValue();

      expect(removedDocumentsWithoutTransaction).to.not.have.lengthOf(0);
      expect(removedDocumentsWithoutTransaction[0].toBuffer()).to.deep.equal(document.toBuffer());

      await documentRepository
        .storage
        .abortTransaction();

      const restoredDocumentsResult = await documentRepository
        .find(dataContract, document.getType(), query);

      const restoredDocuments = restoredDocumentsResult.getValue();

      expect(restoredDocuments).to.not.have.lengthOf(0);
      expect(restoredDocuments[0].toBuffer()).to.deep.equal(document.toBuffer());
    });
  });
});
