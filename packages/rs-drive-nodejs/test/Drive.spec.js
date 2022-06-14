const fs = require('fs');

const { expect, use } = require('chai');
use(require('dirty-chai'));

const Document = require('@dashevo/dpp/lib/document/Document');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const Drive = require('../Drive');

const TEST_DATA_PATH = './test_data';

describe('Drive', () => {
  let drive;
  let dataContract;
  let identity;
  let blockTime;
  let documents;

  beforeEach(() => {
    drive = new Drive(TEST_DATA_PATH);

    dataContract = getDataContractFixture();
    identity = getIdentityFixture();
    blockTime = new Date();
    documents = getDocumentsFixture(dataContract);
  });

  afterEach(async () => {
    await drive.close();

    fs.rmSync(TEST_DATA_PATH, { recursive: true });
  });

  describe('#createRootTree', () => {
    it('should create initial tree structure', async () => {
      const result = await drive.createRootTree();

      // eslint-disable-next-line no-unused-expressions
      expect(result).to.be.undefined;
    });
  });

  describe('#applyContract', () => {
    beforeEach(async () => {
      await drive.createRootTree();
    });

    it('should create contract if not exists', async () => {
      const result = await drive.applyContract(dataContract, blockTime);

      blockTime.setSeconds(blockTime.getSeconds() + 10);

      expect(result).to.have.lengthOf(2);
      expect(result[0]).to.be.greaterThan(0);
      expect(result[1]).to.be.greaterThan(0);
    });

    it('should update existing contract', async () => {
      await drive.applyContract(dataContract, blockTime);

      dataContract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });

      blockTime.setSeconds(blockTime.getSeconds() + 10);

      const result = await drive.applyContract(dataContract, blockTime);

      expect(result).to.have.lengthOf(2);
      expect(result[0]).to.be.greaterThan(0);
      expect(result[1]).to.be.greaterThan(0);
    });
  });

  describe('#createDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract, blockTime);
    });

    context('without indices', () => {
      it('should create a document', async () => {
        const documentWithoutIndices = documents[0];

        const result = await drive.createDocument(documentWithoutIndices, blockTime);

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });

    context('with indices', () => {
      it('should create a document', async () => {
        const documentWithIndices = documents[3];

        const result = await drive.createDocument(documentWithIndices, blockTime);

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });
  });

  describe('#updateDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract, blockTime);
    });

    context('without indices', () => {
      it('should update a document', async () => {
        // Create a document
        const documentWithoutIndices = documents[0];

        await drive.createDocument(documentWithoutIndices, blockTime);

        // Update the document
        documentWithoutIndices.set('name', 'Bob');

        const result = await drive.updateDocument(documentWithoutIndices, blockTime);

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });

    context('with indices', () => {
      it('should update the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockTime);

        // Update the document
        documentWithIndices.set('firstName', 'Bob');

        const result = await drive.updateDocument(documentWithIndices, blockTime);

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });
  });

  describe('#deleteDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract, blockTime);
    });

    context('without indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithoutIndices = documents[3];

        await drive.createDocument(documentWithoutIndices, blockTime);

        const result = await drive.deleteDocument(
          dataContract,
          documentWithoutIndices.getType(),
          documentWithoutIndices.getId(),
        );

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });

    context('with indices', () => {
      it('should delete the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices, blockTime);

        const result = await drive.deleteDocument(
          dataContract,
          documentWithIndices.getType(),
          documentWithIndices.getId(),
        );

        expect(result).to.have.lengthOf(2);
        expect(result[0]).to.be.greaterThan(0);
        expect(result[1]).to.be.greaterThan(0);
      });
    });
  });

  describe('#queryDocuments', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract, blockTime);
    });

    it('should query existing documents', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockTime)),
      );

      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      expect(processingCost).to.be.greaterThan(0);
    });

    it('should query existing documents again', async () => {
      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document, blockTime)),
      );

      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.deep.equal(documents[4].toObject());

      expect(processingCost).to.be.greaterThan(0);
    });

    it('should return empty array if documents are not exist', async () => {
      const [fetchedDocuments, processingCost] = await drive.queryDocuments(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(0);
      expect(processingCost).to.be.greaterThan(0);
    });
  });

  describe('#insertIdentity', () => {
    beforeEach(async () => {
      await drive.createRootTree();
    });

    it('should create identity if not exists', async () => {
      const result = await drive.insertIdentity(identity);

      blockTime.setSeconds(blockTime.getSeconds() + 10);

      expect(result).to.have.lengthOf(2);
      expect(result[0]).to.be.greaterThan(0);
      expect(result[1]).to.be.greaterThan(0);
    });
  });
});
