const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const SVDocument = require('../../../../../lib/stateView/document/SVDocument');
const SVDocumentMongoDbRepository = require('../../../../../lib/stateView/document/mongoDbRepository/SVDocumentMongoDbRepository');

const convertWhereToMongoDbQuery = require('../../../../../lib/stateView/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../../../../../lib/stateView/document/query/validateQueryFactory');
const findConflictingConditions = require('../../../../../lib/stateView/document/query/findConflictingConditions');
const MongoDBTransaction = require('../../../../../lib/mongoDb/MongoDBTransaction');

const getSVDocumentsFixture = require('../../../../../lib/test/fixtures/getSVDocumentsFixture');

const InvalidQueryError = require('../../../../../lib/stateView/document/errors/InvalidQueryError');

function jsonizeSVDocuments(svDocuments) {
  return svDocuments.map(d => d.toJSON());
}

async function createSVDocuments(svDocumentRepository, svDocuments) {
  return Promise.all(
    svDocuments.map(o => svDocumentRepository.store(o)),
  );
}

describe('SVDocumentMongoDbRepository', function main() {
  this.timeout(10000);

  let svDocumentRepository;
  let svDocument;
  let svDocuments;
  let mongoDatabase;
  let mongoClient;
  let stateViewTransaction;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
    mongoClient = mongoDb.getClient();
  });

  beforeEach(async () => {
    svDocuments = getSVDocumentsFixture();

    // Modify documents for the test cases
    svDocuments.forEach((svDoc, i) => {
      const document = svDoc.getDocument();

      document.set('order', i);

      document.set('arrayWithScalar', Array(i + 1)
        .fill(1)
        .map((item, index) => i + index));

      const arrayItem = { item: i + 1, flag: true };
      document.set('arrayWithObjects', Array(i + 1).fill(arrayItem));
    });

    [svDocument] = svDocuments;

    const validateQuery = validateQueryFactory(findConflictingConditions);

    svDocumentRepository = new SVDocumentMongoDbRepository(
      mongoDatabase,
      convertWhereToMongoDbQuery,
      validateQuery,
      svDocument.getDocument().getType(),
    );

    stateViewTransaction = new MongoDBTransaction(mongoClient);
  });

  describe('#store', () => {
    beforeEach(async () => {
      await createSVDocuments(svDocumentRepository, svDocuments);
    });

    it('should store SVDocument', async () => {
      const result = await svDocumentRepository.find(svDocument.getDocument().getId());

      expect(result).to.be.an.instanceOf(SVDocument);
      expect(result.toJSON()).to.deep.equal(svDocument.toJSON());
    });

    it('should store SVDocument in transaction', async () => {
      await svDocumentRepository.delete(svDocument);

      stateViewTransaction.start();

      await svDocumentRepository.store(svDocument, stateViewTransaction);

      const transactionDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId(), stateViewTransaction);
      const notFoundDocument = await svDocumentRepository.find(svDocument.getDocument().getId());

      await stateViewTransaction.commit();

      const createdDocument = await svDocumentRepository.find(svDocument.getDocument().getId());

      expect(notFoundDocument).to.be.a('null');
      expect(transactionDocument).to.be.an.instanceOf(SVDocument);
      expect(transactionDocument.toJSON()).to.deep.equal(svDocument.toJSON());
      expect(createdDocument).to.be.an.instanceOf(SVDocument);
      expect(createdDocument.toJSON()).to.deep.equal(svDocument.toJSON());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await createSVDocuments(svDocumentRepository, svDocuments);
    });

    it('should fetch SVDocuments', async () => {
      const result = await svDocumentRepository.fetch();

      expect(result).to.be.an('array');
      expect(result).to.have.lengthOf(3);

      const actualRawSVDocuments = jsonizeSVDocuments(result);
      const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

      expect(actualRawSVDocuments).to.have.deep.members(expectedRawSVDocuments);
    });

    it('should fetch SVDocuments in transaction', async () => {
      stateViewTransaction.start();

      const result = await svDocumentRepository.fetch({}, stateViewTransaction);

      await stateViewTransaction.commit();

      expect(result).to.be.an('array');
      expect(result).to.have.lengthOf(3);

      const actualRawSVDocuments = jsonizeSVDocuments(result);
      const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

      expect(actualRawSVDocuments).to.have.deep.members(expectedRawSVDocuments);
    });

    it('should throw InvalidQueryError if query is not valid', async () => {
      const invalidQuery = { invalid: 'query' };

      let error;
      try {
        await svDocumentRepository.fetch(invalidQuery);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidQueryError);
      expect(error.getErrors()).has.lengthOf(1);
    });

    it('should not fetch SVDocument that is marked as deleted');

    describe('where', () => {
      it('should find SVDocuments using "<" operator', async () => {
        const query = {
          where: [['order', '<', svDocuments[1].getDocument().get('order')]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[0].toJSON());
      });

      it('should find SVDocuments using "<=" operator', async () => {
        const query = {
          where: [['order', '<=', svDocuments[1].getDocument().get('order')]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const actualRawSVDocuments = jsonizeSVDocuments(result);

        svDocuments.pop();
        const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

        expect(actualRawSVDocuments).to.deep.members(expectedRawSVDocuments);
      });

      it('should find SVDocuments using "==" operator', async () => {
        const query = {
          where: [['name', '==', svDocument.getDocument().get('name')]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocument.toJSON());
      });

      it('should find SVDocuments using ">" operator', async () => {
        const query = {
          where: [['order', '>', svDocuments[1].getDocument().get('order')]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[2].toJSON());
      });

      it('should find SVDocuments using ">=" operator', async () => {
        const query = {
          where: [['order', '>=', svDocuments[1].getDocument().get('order')]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const actualRawSVDocuments = jsonizeSVDocuments(result);

        svDocuments.shift();
        const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

        expect(actualRawSVDocuments).to.have.deep.members(expectedRawSVDocuments);
      });

      it('should find SVDocuments using "in" operator', async () => {
        const query = {
          where: [
            ['$id', 'in', [
              svDocuments[0].getDocument().getId(),
              svDocuments[1].getDocument().getId(),
            ]],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const actualRawSVDocuments = jsonizeSVDocuments(result);

        svDocuments.pop();
        const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

        expect(actualRawSVDocuments).to.have.deep.members(expectedRawSVDocuments);
      });

      it('should find SVDocuments using "length" operator', async () => {
        const query = {
          where: [['arrayWithObjects', 'length', 2]],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[1].toJSON());
      });

      it('should find SVDocuments using "startsWith" operator', async () => {
        const query = {
          where: [['lastName', 'startsWith', 'Swe']],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[2].toJSON());
      });

      it('should find SVDocuments using "elementMatch" operator', async () => {
        const query = {
          where: [
            ['arrayWithObjects', 'elementMatch', [
              ['item', '==', 2], ['flag', '==', true],
            ]],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[1].toJSON());
      });

      it('should find SVDocuments using "contains" operator and array value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', [2, 3]],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[2].toJSON());
      });

      it('should find SVDocuments using "contains" operator and scalar value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', 2],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const actualRawSVDocuments = jsonizeSVDocuments(result);

        svDocuments.shift();
        const expectedRawSVDocuments = jsonizeSVDocuments(svDocuments);

        expect(actualRawSVDocuments).to.have.deep.members(expectedRawSVDocuments);
      });

      it('should return empty array if where clause conditions do not match', async () => {
        const query = {
          where: [['name', '==', 'Dash enthusiast']],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.deep.equal([]);
      });

      it('should find SVDocuments by nested object fields', async () => {
        const query = {
          where: [
            ['arrayWithObjects.item', '==', 2],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[1].toJSON());
      });

      it('should return SVDocuments by several conditions', async () => {
        const query = {
          where: [
            ['name', '==', 'Cutie'],
            ['arrayWithObjects', 'elementMatch', [
              ['item', '==', 1],
              ['flag', '==', true],
            ]],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedSVDocument] = result;

        expect(expectedSVDocument.toJSON()).to.deep.equal(svDocuments[0].toJSON());
      });
    });

    describe('limit', () => {
      it('should limit return to 1 SVDocument if limit is set', async () => {
        const options = {
          limit: 1,
        };

        const result = await svDocumentRepository.fetch(options);

        expect(result).to.be.an('array');
        expect(result).to.have.lengthOf(1);
      });

      it('should limit result to 100 SVDocuments if limit is not set', async () => {
        // Store 101 document
        await Promise.all(
          Array(101).fill(svDocument).map((svDoc, i) => {
            // Ensure unique ID

            // eslint-disable-next-line no-param-reassign
            svDoc.getDocument().id = i + 1;

            return svDocumentRepository.store(svDoc);
          }),
        );

        const result = await svDocumentRepository.fetch();

        expect(result).to.be.an('array');
        expect(result).to.have.lengthOf(100);
      });
    });

    describe('startAt', () => {
      it('should return SVDocuments from 2 document', async () => {
        const query = {
          orderBy: [
            ['order', 'asc'],
          ],
          startAt: 2,
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');

        const actualRawSVDocuments = result.map(d => d.toJSON());
        const expectedRawSVDocuments = svDocuments.splice(1).map(d => d.toJSON());

        expect(actualRawSVDocuments).to.deep.equal(expectedRawSVDocuments);
      });
    });

    describe('startAfter', () => {
      it('should return SVDocuments after 1 document', async () => {
        const options = {
          orderBy: [
            ['order', 'asc'],
          ],
          startAfter: 1,
        };

        const result = await svDocumentRepository.fetch(options);

        expect(result).to.be.an('array');

        const actualRawSVDocuments = result.map(d => d.toJSON());
        const expectedRawSVDocuments = svDocuments.splice(1).map(d => d.toJSON());

        expect(actualRawSVDocuments).to.deep.equal(expectedRawSVDocuments);
      });
    });

    describe('orderBy', () => {
      it('should sort SVDocuments in descending order', async () => {
        const query = {
          orderBy: [
            ['order', 'desc'],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');

        const actualRawSVDocuments = result.map(d => d.toJSON());
        const expectedRawSVDocuments = svDocuments.reverse().map(d => d.toJSON());

        expect(actualRawSVDocuments).to.deep.equal(expectedRawSVDocuments);
      });

      it('should sort SVDocuments in ascending order', async () => {
        const query = {
          orderBy: [
            ['order', 'asc'],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');

        const actualRawSVDocuments = result.map(d => d.toJSON());
        const expectedRawSVDocuments = svDocuments.map(d => d.toJSON());

        expect(actualRawSVDocuments).to.deep.equal(expectedRawSVDocuments);
      });

      it('should sort SVDocuments using two fields', async () => {
        svDocuments[0].getDocument().set('primaryOrder', 1);
        svDocuments[1].getDocument().set('primaryOrder', 2);
        svDocuments[2].getDocument().set('primaryOrder', 2);

        await Promise.all(
          svDocuments.map(o => svDocumentRepository.store(o)),
        );

        const query = {
          orderBy: [
            ['primaryOrder', 'asc'],
            ['order', 'desc'],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(3);

        expect(result[0].toJSON()).to.deep.equal(svDocuments[0].toJSON());
        expect(result[1].toJSON()).to.deep.equal(svDocuments[2].toJSON());
        expect(result[2].toJSON()).to.deep.equal(svDocuments[1].toJSON());
      });

      it('should sort SVDocuments by $id', async () => {
        await Promise.all(
          svDocuments.map(d => svDocumentRepository.delete(d)),
        );

        await Promise.all(
          svDocuments.map((svDoc, i) => {
            // eslint-disable-next-line no-param-reassign
            svDoc.getDocument().id = i + 1;

            return svDocumentRepository.store(svDoc);
          }),
        );

        const query = {
          orderBy: [
            ['$id', 'desc'],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(3);

        expect(result[0].toJSON()).to.deep.equal(svDocuments[2].toJSON());
        expect(result[1].toJSON()).to.deep.equal(svDocuments[1].toJSON());
        expect(result[2].toJSON()).to.deep.equal(svDocuments[0].toJSON());
      });

      it('should sort SVDocuments by $userId', async () => {
        await Promise.all(
          svDocuments.map((svDoc, i) => {
            svDoc.setUserId(i + 1);
            // eslint-disable-next-line no-param-reassign
            svDoc.getDocument().getMetadata().userId = i + 1;

            return svDocumentRepository.store(svDoc);
          }),
        );

        const query = {
          orderBy: [
            ['$userId', 'desc'],
          ],
        };

        const result = await svDocumentRepository.fetch(query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(3);

        expect(result[0].toJSON()).to.deep.equal(svDocuments[2].toJSON());
        expect(result[1].toJSON()).to.deep.equal(svDocuments[1].toJSON());
        expect(result[2].toJSON()).to.deep.equal(svDocuments[0].toJSON());
      });
    });
  });

  describe('#delete', () => {
    beforeEach(async () => {
      await createSVDocuments(svDocumentRepository, svDocuments);
    });

    it('should delete SVDocument', async () => {
      await svDocumentRepository.delete(svDocument);

      const result = await svDocumentRepository.find(svDocument.getDocument().getId());

      expect(result).to.be.null();
    });

    it('should delete SVDocument in transaction', async () => {
      stateViewTransaction.start();

      await svDocumentRepository.delete(svDocument, stateViewTransaction);

      const removedDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId(), stateViewTransaction);

      const notRemovedDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId());

      await stateViewTransaction.commit();

      const completelyRemovedDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId());

      expect(removedDocument).to.be.a('null');
      expect(notRemovedDocument).to.be.an.instanceOf(SVDocument);
      expect(notRemovedDocument.toJSON()).to.deep.equal(svDocument.toJSON());
      expect(completelyRemovedDocument).to.be.a('null');
    });

    it('should restore document if transaction aborted', async () => {
      stateViewTransaction.start();

      await svDocumentRepository.delete(svDocument, stateViewTransaction);

      const removedDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId(), stateViewTransaction);

      const notRemovedDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId());

      await stateViewTransaction.abort();

      const restoredDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId());

      expect(removedDocument).to.be.a('null');
      expect(notRemovedDocument).to.be.an.instanceOf(SVDocument);
      expect(notRemovedDocument.toJSON()).to.deep.equal(svDocument.toJSON());
      expect(restoredDocument).to.be.an.instanceOf(SVDocument);
      expect(restoredDocument.toJSON()).to.deep.equal(svDocument.toJSON());
    });
  });

  describe('#find', () => {
    beforeEach(async () => {
      await createSVDocuments(svDocumentRepository, svDocuments);
    });

    it('should find SVDocument by ID');

    it('should find SVDocument marked as deleted by ID');

    it('should return SVDocument with Document having proper $meta', async () => {
      await svDocumentRepository.store(svDocument);

      const foundSVDocument = await svDocumentRepository
        .find(svDocument.getDocument().getId());

      const metadataJSON = foundSVDocument.getDocument().getMetadata()
        .toJSON();

      const currentReference = svDocument.getCurrentRevision()
        .getReference();

      expect(metadataJSON).to.deep.equal({
        userId: svDocument.getUserId(),
        stReference: {
          blockHash: currentReference.getBlockHash(),
          blockHeight: currentReference.getBlockHeight(),
          stHeaderHash: currentReference.getSTHash(),
          stPacketHash: currentReference.getSTPacketHash(),
        },
      });
    });

    it('should return null if SVDocument was not found', async () => {
      const document = await svDocumentRepository.find('unknown');

      expect(document).to.be.null();
    });
  });

  describe('#createCollection', () => {
    it('should create collection', async () => {
      const collectionsBefore = await mongoDatabase.listCollections().toArray();
      await svDocumentRepository.createCollection();
      const collectionsAfter = await mongoDatabase.listCollections().toArray();

      expect(collectionsBefore).to.have.length(0);
      expect(collectionsAfter).to.have.length(1);
      expect(collectionsAfter[0].name).to.equal(svDocumentRepository.getCollectionName());
    });
  });
});
