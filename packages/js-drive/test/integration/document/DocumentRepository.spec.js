const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const createDocumentTypeTreePath = require('../../../lib/document/groveDB/createDocumentTreePath');
const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

async function createDocuments(documentRepository, documents) {
  return Promise.all(
    documents.map((o) => documentRepository.store(o)),
  );
}

describe('DocumentRepository', function main() {
  this.timeout(30000);

  let documentRepository;
  let container;
  let dataContract;
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
      //   // {
      //   //   name: 'index2',
      //   //
      //   //   properties: [{ name: 'asc' }, { 'arrayWithObjects.item': 'asc' }],
      //   // },
      {
        name: 'index3',
        properties: [{ order: 'asc' }],
      },
      {
        name: 'index4',
        properties: [{ lastName: 'asc' }],
      },
      //   // {
      //   //   name: 'index5',
      //   //   properties: [{ arrayWithScalar: 'asc' }],
      //   // },
      //   // {
      //   //   name: 'index6',
      //   //   properties: [{ arrayWithObjects: 'asc' }],
      //   // },
      //   // {
      //   //   name: 'index7',
      //   //   properties: [{ 'arrayWithObjects.item': 'asc' }],
      //   // },
      //   // {
      //   //   name: 'index8',
      //   //   properties: [{ 'arrayWithObjects.flag': 'asc' }],
      //   // },
      //   // {
      //   //   name: 'index9',
      //   //   properties: [{ primaryOrder: 'asc' }, { order: 'desc' }],
      //   // },
      {
        name: 'index10',
        properties: [{ $ownerId: 'desc' }],
      },
    ]);

    documentRepository = container.resolve('documentRepository');

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    const dataContractRepository = container.resolve('dataContractRepository');

    await dataContractRepository.store(dataContract);
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
        expect(e.message.indexOf('invalid path key: key not found in Merk') !== -1).to.be.true();
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

    it('should find Documents', async () => {
      const foundDocuments = await documentRepository.find(dataContract, document.getType());

      expect(foundDocuments).to.be.an('array');
      expect(foundDocuments).to.have.lengthOf(documents.length);

      const foundDocumentsBuffers = foundDocuments.map((doc) => doc.toBuffer());

      expect(foundDocumentsBuffers).to.have.deep.members(documents.map((doc) => doc.toBuffer()));
    });

    it('should fetch Documents in transaction', async () => {
      await documentRepository
        .storage
        .startTransaction();

      const foundDocuments = await documentRepository
        .find(dataContract, document.getType(), {}, true);

      await documentRepository.storage.commitTransaction();

      expect(foundDocuments).to.be.an('array');
      expect(foundDocuments).to.have.lengthOf(documents.length);

      const foundDocumentsBuffers = foundDocuments.map((doc) => doc.toBuffer());

      expect(foundDocumentsBuffers).to.have.deep.members(documents.map((doc) => doc.toBuffer()));
    });

    it('should throw InvalidQueryError if query is not valid', async () => {
      const invalidQuery = { invalid: 'query' };

      let error;
      try {
        await documentRepository.find(dataContract, document.getType(), invalidQuery);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidQueryError);
      expect(error.getErrors()).has.lengthOf(1);
    });

    describe('where', () => {
      it('should find Documents using "<" operator', async () => {
        const query = {
          where: [['order', '<', documents[1].get('order')]],
          orderBy: [['order', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(documents[0].toBuffer());
      });

      it('should find Documents using "<=" operator', async () => {
        const query = {
          where: [['order', '<=', documents[1].get('order')]],
          orderBy: [['order', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedDocuments = documents.slice(0, 2).map((doc) => doc.toBuffer());

        expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
      });

      it('should find Documents using "==" operator', async () => {
        const query = {
          where: [['name', '==', document.get('name')]],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(document.toBuffer());
      });

      it('should find Documents using ">" operator', async () => {
        const query = {
          where: [['order', '>', documents[1].get('order')]],
          orderBy: [['order', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length - 2);

        const expectedDocuments = documents
          .splice(2, documents.length)
          .map((doc) => doc.toBuffer());

        expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
      });

      it('should find Documents using ">=" operator', async () => {
        const query = {
          where: [['order', '>=', documents[1].get('order')]],
          orderBy: [['order', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(documents.length - 1);

        documents.shift();
        const expectedDocuments = documents
          .map((doc) => doc.toBuffer());

        expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
      });

      it('should find Documents using "in" operator', async () => {
        const query = {
          where: [
            ['$id', 'in', [
              documents[0].getId(),
              documents[1].getId(),
            ]],
          ],
          orderBy: [['$id', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedDocuments = documents.slice(0, 2).map((doc) => doc.toBuffer());

        expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
      });

      it.skip('should find Documents using "length" operator', async () => {
        const query = {
          where: [['arrayWithObjects', 'length', 2]],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(documents[1].toBuffer());
      });

      it('should find Documents using "startsWith" operator', async () => {
        const query = {
          where: [['lastName', 'startsWith', 'Swe']],
          orderBy: [['lastName', 'asc']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(documents[2].toBuffer());
      });

      it.skip('should find Documents using "elementMatch" operator', async () => {
        const query = {
          where: [
            ['arrayWithObjects', 'elementMatch', [
              ['item', '==', 2], ['flag', '==', true],
            ]],
          ],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(documents[1].toBuffer());
      });

      it.skip('should find Documents using "contains" operator and array value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', [2, 3]],
          ],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(1);

        const [expectedDocument] = result;

        expect(expectedDocument.toBuffer()).to.deep.equal(documents[2].toBuffer());
      });

      it.skip('should find Documents using "contains" operator and scalar value', async () => {
        const query = {
          where: [
            ['arrayWithScalar', 'contains', 2],
          ],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.be.an('array');
        expect(result).to.be.lengthOf(2);

        const expectedDocuments = documents.slice(1, 3).map((doc) => doc.toBuffer());

        expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
      });

      it('should return empty array if where clause conditions do not match', async () => {
        const query = {
          where: [['name', '==', 'Dash enthusiast']],
        };

        const result = await documentRepository.find(dataContract, document.getType(), query);

        expect(result).to.have.lengthOf(0);
      });

      it.skip('should find Documents by nested object fields', async () => {
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

      it.skip('should return Documents by several conditions', async () => {
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

      describe('limit', () => {
        it('should limit return to 1 Document if limit is set', async () => {
          const options = {
            limit: 1,
          };

          const result = await documentRepository.find(dataContract, document.getType(), options);

          expect(result).to.be.an('array');
          expect(result).to.have.lengthOf(1);
        });

        it('should limit result to 100 Documents if limit is not set', async () => {
          // Store 101 document
          for (let i = 0; i < 101; i++) {
            const svDoc = document;

            svDoc.id = Identifier.from(Buffer.alloc(32, i + 1));
            await documentRepository.store(svDoc);
          }

          const result = await documentRepository.find(dataContract, document.getType());
          expect(result).to.be.an('array');
          expect(result).to.have.lengthOf(100);
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

          const result = await documentRepository.find(dataContract, document.getType(), query);

          expect(result).to.be.an('array');

          const expectedDocuments = documents.splice(1).map((doc) => doc.toBuffer());

          expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
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

          expect(result).to.be.an('array');

          const expectedDocuments = documents.splice(1).map((doc) => doc.toBuffer());

          expect(result.map((doc) => doc.toBuffer())).to.deep.members(expectedDocuments);
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

          expect(result).to.be.an('array');

          const expectedDocuments = documents.reverse().map((doc) => doc.toBuffer());

          expect(result.map((doc) => doc.toBuffer())).to.deep.equal(expectedDocuments);
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

          expect(result).to.be.an('array');

          const expectedDocuments = documents.map((doc) => doc.toBuffer());

          expect(result.map((doc) => doc.toBuffer())).to.deep.equal(expectedDocuments);
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

          expect(result).to.be.an('array');
          expect(result).to.be.lengthOf(documents.length);

          expect(result[0].getId()).to.deep.equal(createdIds[4]);
          expect(result[1].getId()).to.deep.equal(createdIds[3]);
          expect(result[2].getId()).to.deep.equal(createdIds[2]);
          expect(result[3].getId()).to.deep.equal(createdIds[1]);
          expect(result[4].getId()).to.deep.equal(createdIds[0]);
        });
      });
    });
  });

  describe('#delete', () => {
    beforeEach(async () => {
      await createDocuments(documentRepository, documents);
    });

    it('should delete Document', async () => {
      await documentRepository.delete(dataContract, document.getType(), document.getId());

      const result = await documentRepository.find(dataContract, document.getType(), {
        where: [['$id', '==', document.getId()]],
      });

      expect(result).to.have.lengthOf(0);
    });

    it('should delete Document in transaction', async () => {
      await documentRepository
        .storage
        .startTransaction();

      await documentRepository.delete(
        dataContract,
        document.getType(),
        document.getId(),
        true,
      );

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const removedDocument = await documentRepository
        .find(
          dataContract,
          document.getType(),
          query,
          true,
        );

      const notRemovedDocuments = await documentRepository
        .find(dataContract, document.getType(), query);

      await documentRepository
        .storage.commitTransaction();

      const completelyRemovedDocument = await documentRepository
        .find(dataContract, document.getType(), query);

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

      const removedDocuments = await documentRepository
        .find(dataContract, document.getType(), query, true);

      const notRemovedDocuments = await documentRepository
        .find(dataContract, document.getType(), query);

      await documentRepository
        .storage
        .abortTransaction();

      const restoredDocuments = await documentRepository
        .find(dataContract, document.getType(), query);

      expect(removedDocuments).to.have.lengthOf(0);
      expect(notRemovedDocuments).to.be.not.null();
      expect(notRemovedDocuments[0].toBuffer()).to.deep.equal(document.toBuffer());
      expect(restoredDocuments).to.be.not.null();
      expect(restoredDocuments[0].toBuffer()).to.deep.equal(document.toBuffer());
    });
  });
});
