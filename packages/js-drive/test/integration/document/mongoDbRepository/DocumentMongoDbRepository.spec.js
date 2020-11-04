const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const Identifier = require('@dashevo/dpp/lib/Identifier');

const InvalidQueryError = require('../../../../lib/document/errors/InvalidQueryError');

const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');

function getIds(documents) {
  return documents.map((d) => d.getId());
}

async function createDocuments(documentRepository, documents) {
  return Promise.all(
    documents.map((o) => documentRepository.store(o)),
  );
}

describe('DocumentMongoDbRepository', function main() {
  this.timeout(10000);

  let documentRepository;
  let document;
  let documents;
  let documentSchema;
  let dataContract;
  let mongoDb;
  let container;
  let createDocumentMongoDbRepository;
  let documentMongoDBTransaction;

  startMongoDb().then((mongo) => {
    mongoDb = mongo;
  });

  beforeEach(async () => {
    container = await createTestDIContainer(mongoDb);

    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract).slice(0, 5);

    [document] = documents;

    // Modify documents for the test cases
    documents = documents.map((doc, i) => {
      const currentDocument = doc;
      const arrayItem = { item: i + 1, flag: true };

      currentDocument.set('order', i);
      currentDocument.set('arrayWithScalar', Array(i + 1)
        .fill(1)
        .map((item, index) => i + index));
      currentDocument.set('arrayWithObjects', Array(i + 1).fill(arrayItem));
      currentDocument.type = document.getType();

      return currentDocument;
    });

    [document] = documents;

    dataContract.documents[document.getType()].properties = {
      ...dataContract.documents[document.getType()].properties,
      order: {
        type: 'number',
      },
      lastName: {
        type: 'string',
      },
      arrayWithScalar: {
        type: 'array',
        items: [
          { type: 'string' },
        ],
      },
      arrayWithObjects: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            flag: {
              type: 'string',
            },
          },
        },
      },
    };

    const documentsSchema = dataContract.getDocuments();

    documentSchema = documentsSchema[document.getType()];

    // redeclare indices
    const indices = documentSchema.indices || [];
    documentSchema.indices = indices.concat([
      {
        properties: [{ name: 'asc' }],
      },
      {
        properties: [{ order: 'asc' }],
      },
      {
        properties: [{ lastName: 'asc' }],
      },
      {
        properties: [{ arrayWithScalar: 'asc' }],
      },
      {
        properties: [{ arrayWithObjects: 'asc' }],
      },
      {
        properties: [{ 'arrayWithObjects.item': 'asc' }],
      },
      {
        properties: [{ 'arrayWithObjects.flag': 'asc' }],
      },
      {
        properties: [{ primaryOrder: 'asc' }, { order: 'desc' }],
      },
      {
        properties: [{ $ownerId: 'desc' }],
      },
    ]);

    createDocumentMongoDbRepository = container.resolve('createDocumentMongoDbRepository');
    documentMongoDBTransaction = container.resolve('documentMongoDBTransaction');
    const dataContractRepository = container.resolve('dataContractRepository');
    const documentDatabaseManager = container.resolve('documentDatabaseManager');

    await dataContractRepository.store(dataContract);
    await documentDatabaseManager.create(dataContract);

    documentRepository = await createDocumentMongoDbRepository(
      dataContract.getId(),
      document.getType(),
    );
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
      const findQuery = {
        _id: {
          $eq: document.getId(),
        },
      };

      const findOptions = {
        promoteBuffers: true,
        projection: {
          _id: 1,
        },
        limit: 100,
      };

      const result = await documentRepository
        .mongoCollection
        .find(findQuery, findOptions)
        .toArray();

      expect(result).to.have.lengthOf(1);
      // eslint-disable-next-line no-underscore-dangle
      const [documentId] = result.map((mongoDbDocument) => new Identifier(mongoDbDocument._id));

      expect(documentId).to.deep.equal(document.getId());
    });

    it('should store Document in transaction', async () => {
      await documentRepository.delete(document.getId());

      await documentMongoDBTransaction.start();

      await documentRepository.store(document, documentMongoDBTransaction);

      const transactionDocumentIds = await documentRepository
        .find({
          where: [['$id', '==', document.getId()]],
        }, documentMongoDBTransaction);
      const notFoundDocument = await documentRepository.find({
        where: [['$id', '==', document.getId()]],
      });

      await documentMongoDBTransaction.commit();

      const createdDocumentIds = await documentRepository.find({
        where: [['$id', '==', document.getId()]],
      });

      expect(notFoundDocument).to.have.lengthOf(0);
      expect(transactionDocumentIds).to.have.lengthOf(1);

      const [transactionDocumentId] = transactionDocumentIds;

      expect(transactionDocumentId).to.be.an.instanceOf(Identifier);
      expect(transactionDocumentId).to.deep.equal(document.getId());

      expect(createdDocumentIds).to.have.lengthOf(1);

      const [createdDocumentId] = createdDocumentIds;
      expect(createdDocumentId).to.deep.equal(document.getId());
    });
  });

  describe('#find', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should find Document ids', async () => {
      const documentIds = await documentRepository.find();

      expect(documentIds).to.be.an('array');
      expect(documentIds).to.have.lengthOf(documents.length);

      expect(documentIds).to.have.deep.members(
        documents.map((doc) => doc.getId()),
      );
    });

    it('should fetch Document ids in transaction', async () => {
      await documentMongoDBTransaction.start();

      const result = await documentRepository.find(
        {},
        documentMongoDBTransaction,
      );

      await documentMongoDBTransaction.commit();

      expect(result).to.be.an('array');
      expect(result).to.have.lengthOf(documents.length);

      const expectedIds = documents.map((doc) => doc.getId());

      expect(result).to.have.deep.members(expectedIds);
    });

    it('should throw InvalidQueryError if query is not valid', async () => {
      const invalidQuery = { invalid: 'query' };

      let error;
      try {
        await documentRepository.find(invalidQuery);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidQueryError);
      expect(error.getErrors()).has.lengthOf(1);
    });

    describe('where', () => {
      it('should find Document ids using "<" operator', async () => {
        const query = {
          where: [['order', '<', documents[1].get('order')]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[0].getId());
      });

      it('should find Document ids using "<=" operator', async () => {
        const query = {
          where: [['order', '<=', documents[1].get('order')]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedIds = getIds(documents.slice(0, 2));

        expect(result).to.deep.members(expectedIds);
      });

      it('should find Document ids using "==" operator', async () => {
        const query = {
          where: [['name', '==', document.get('name')]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedId] = result;

        expect(expectedId).to.deep.equal(document.getId());
      });

      it('should find Document ids using ">" operator', async () => {
        const query = {
          where: [['order', '>', documents[1].get('order')]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length - 2);

        const expectedIds = getIds(documents.splice(2, documents.length));

        expect(result).to.have.deep.members(expectedIds);
      });

      it('should find Document ids using ">=" operator', async () => {
        const query = {
          where: [['order', '>=', documents[1].get('order')]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length - 1);

        documents.shift();
        const expectedIds = getIds(documents);

        expect(result).to.have.deep.members(expectedIds);
      });

      it('should find Document ids using "in" operator', async () => {
        const query = {
          where: [
            ['$id', 'in', [
              documents[0].getId(),
              documents[1].getId(),
            ]],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedIds = getIds(documents.slice(0, 2));

        expect(result).to.have.deep.members(expectedIds);
      });

      it('should find Document ids using "length" operator', async () => {
        const query = {
          where: [['arrayWithObjects', 'length', 2]],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[1].getId());
      });

      it('should find Document ids using "startsWith" operator', async () => {
        const query = {
          where: [['lastName', 'startsWith', 'Swe']],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[2].getId());
      });

      it('should find Document ids using "elementMatch" operator', async () => {
        const query = {
          where: [
            ['arrayWithObjects', 'elementMatch', [
              ['item', '==', 2], ['flag', '==', true],
            ]],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[1].getId());
      });

      it('should find Document ids using "contains" operator and array value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', [2, 3]],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[2].getId());
      });

      it('should find Document ids using "contains" operator and scalar value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', 2],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedIds = getIds(documents.slice(1, 3));

        expect(result).to.have.deep.members(expectedIds);
      });

      it('should return empty array if where clause conditions do not match', async () => {
        const query = {
          where: [['name', '==', 'Dash enthusiast']],
        };

        const result = await documentRepository.find(query);

        expect(result).to.have.lengthOf(0);
      });

      it('should find Document ids by nested object fields', async () => {
        const query = {
          where: [
            ['arrayWithObjects.item', '==', 2],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentIds] = result;

        expect(expectedDocumentIds).to.deep.equal(documents[1].getId());
      });

      it('should return Document ids by several conditions', async () => {
        const query = {
          where: [
            ['name', '==', 'Cutie'],
            ['arrayWithObjects', 'elementMatch', [
              ['item', '==', 1],
              ['flag', '==', true],
            ]],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocumentId] = result;

        expect(expectedDocumentId).to.deep.equal(documents[0].getId());
      });
    });

    describe('limit', () => {
      it('should limit return to 1 Document if limit is set', async () => {
        const options = {
          limit: 1,
        };

        const result = await documentRepository.find(options);

        expect(result).to.be.an('array');
        expect(result).to.have.lengthOf(1);
      });

      it('should limit result to 100 Documents if limit is not set', async () => {
        // Store 101 document
        await Promise.all(
          Array(101).fill(document).map((svDoc, i) => {
            // Ensure unique ID

            // eslint-disable-next-line no-param-reassign
            svDoc.id = Identifier.from(Buffer.alloc(32, i + 1));

            return documentRepository.store(svDoc);
          }),
        );

        const result = await documentRepository.find();

        expect(result).to.be.an('array');
        expect(result).to.have.lengthOf(100);
      });
    });

    describe('startAt', () => {
      it('should return Document ids from 2 document', async () => {
        const query = {
          orderBy: [
            ['order', 'asc'],
          ],
          startAt: 2,
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');

        const expectedIds = getIds(documents.splice(1));

        expect(result).to.deep.equal(expectedIds);
      });
    });

    describe('startAfter', () => {
      it('should return Document ids after 1 document', async () => {
        const options = {
          orderBy: [
            ['order', 'asc'],
          ],
          startAfter: 1,
        };

        const result = await documentRepository.find(options);

        expect(result).to.be.an('array');

        const expectedIds = getIds(documents.splice(1));

        expect(result).to.deep.equal(expectedIds);
      });
    });

    describe('orderBy', () => {
      it('should sort Document ids in descending order', async () => {
        const query = {
          orderBy: [
            ['order', 'desc'],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');

        const expectedIds = getIds(documents.reverse());

        expect(result).to.deep.equal(expectedIds);
      });

      it('should sort Document ids in ascending order', async () => {
        const query = {
          orderBy: [
            ['order', 'asc'],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');

        const expectedIds = getIds(documents);

        expect(result).to.deep.equal(expectedIds);
      });

      it('should sort Document ids using two fields', async () => {
        documents[0].set('primaryOrder', 1);
        documents[1].set('primaryOrder', 2);
        documents[2].set('primaryOrder', 2);
        documents[3].set('primaryOrder', 3);
        documents[4].set('primaryOrder', 4);

        await Promise.all(
          documents.map((o) => documentRepository.store(o)),
        );

        const query = {
          orderBy: [
            ['primaryOrder', 'asc'],
            ['order', 'desc'],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length);

        expect(result[0]).to.deep.equal(documents[0].getId());
        expect(result[1]).to.deep.equal(documents[2].getId());
        expect(result[2]).to.deep.equal(documents[1].getId());
        expect(result[3]).to.deep.equal(documents[3].getId());
        expect(result[4]).to.deep.equal(documents[4].getId());
      });

      it('should sort Documents by $id', async () => {
        await Promise.all(
          documents.map((d) => documentRepository.delete(d.getId())),
        );

        await Promise.all(
          documents.map((svDoc, i) => {
            // eslint-disable-next-line no-param-reassign
            svDoc.id = Identifier.from(Buffer.alloc(32, i + 1));

            return documentRepository.store(svDoc);
          }),
        );

        const query = {
          orderBy: [
            ['$id', 'desc'],
          ],
        };

        const result = await documentRepository.find(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length);

        expect(result[0]).to.deep.equal(documents[4].getId());
        expect(result[1]).to.deep.equal(documents[3].getId());
        expect(result[2]).to.deep.equal(documents[2].getId());
        expect(result[3]).to.deep.equal(documents[1].getId());
        expect(result[4]).to.deep.equal(documents[0].getId());
      });
    });
  });

  describe('#delete', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should delete Document', async () => {
      await documentRepository.delete(document.getId());

      const result = await documentRepository.find({
        where: [['$id', '==', document.getId()]],
      });

      expect(result).to.have.lengthOf(0);
    });

    it('should delete Document in transaction', async () => {
      await documentMongoDBTransaction.start();

      await documentRepository.delete(
        document.getId(),
        documentMongoDBTransaction,
      );

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const removedDocumentId = await documentRepository
        .find(query, documentMongoDBTransaction);

      const notRemovedDocumentIds = await documentRepository
        .find(query);

      await documentMongoDBTransaction.commit();

      const completelyRemovedDocumentId = await documentRepository
        .find(query);

      expect(removedDocumentId).to.have.lengthOf(0);
      expect(notRemovedDocumentIds).to.be.not.null();
      expect(notRemovedDocumentIds[0]).to.deep.equal(document.getId());
      expect(completelyRemovedDocumentId).to.have.lengthOf(0);
    });

    it('should restore document if transaction aborted', async () => {
      await documentMongoDBTransaction.start();

      await documentRepository.delete(document.getId(), documentMongoDBTransaction);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const removedDocumentIds = await documentRepository
        .find(query, documentMongoDBTransaction);

      const notRemovedDocumentIds = await documentRepository
        .find(query);

      await documentMongoDBTransaction.abort();

      const restoredDocumentIds = await documentRepository
        .find(query);

      expect(removedDocumentIds).to.have.lengthOf(0);
      expect(notRemovedDocumentIds).to.be.not.null();
      expect(notRemovedDocumentIds[0]).to.deep.equal(document.getId());
      expect(restoredDocumentIds).to.be.not.null();
      expect(restoredDocumentIds[0]).to.deep.equal(document.getId());
    });
  });

  describe('#removeCollection', () => {
    let mongoDbDatabase;

    beforeEach(async () => {
      await createDocuments(documentRepository, documents);

      const getDocumentMongoDBDatabase = container.resolve('getDocumentMongoDBDatabase');

      mongoDbDatabase = await getDocumentMongoDBDatabase(dataContract.getId());
    });

    it('should remove collection for Document', async () => {
      const collectionsBefore = await mongoDbDatabase.collections();
      const result = await documentRepository.removeCollection();
      const collectionsAfter = await mongoDbDatabase.collections();

      expect(result).to.be.true();
      expect(collectionsAfter).to.have.lengthOf(collectionsBefore.length - 1);
    });
  });
});
