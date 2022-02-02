const fs = require('fs');

const { expect } = require('chai');

const Document = require('@dashevo/dpp/lib/document/Document');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const Drive = require('./index');

const TEST_DATA_PATH = './test_data';

describe('Drive', () => {
  let drive;
  let dataContract;
  let documents;

  beforeEach(() => {
    drive = new Drive(TEST_DATA_PATH);

    dataContract = getDataContractFixture();
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
      const result = await drive.applyContract(dataContract);

      expect(result).to.equals(0);
    });

    it('should update existing contract', async () => {
      await drive.applyContract(dataContract);

      dataContract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });

      const result = await drive.applyContract(dataContract);

      expect(result).to.equals(0);
    });
  });

  describe('#createDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract);
    });

    context('without indices', () => {
      it('should create a document', async () => {
        const documentWithoutIndices = documents[0];

        const result = await drive.createDocument(documentWithoutIndices);

        expect(result).to.equals(0);
      });
    });

    context('with indices', () => {
      it('should create a document', async () => {
        const documentWithIndices = documents[3];

        const result = await drive.createDocument(documentWithIndices);

        expect(result).to.equals(0);
      });
    });
  });

  describe('#updateDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract);
    });

    context('without indices', () => {
      it('should should update a document', async () => {
        // Create a document
        const documentWithoutIndices = documents[0];

        await drive.createDocument(documentWithoutIndices);

        // Update the document
        documentWithoutIndices.set('name', 'Bob');

        const result = await drive.updateDocument(documentWithoutIndices);

        expect(result).to.equals(0);
      });
    });

    context('with indices', () => {
      it('should should update the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices);

        // Update the document
        documentWithIndices.set('firstName', 'Bob');

        const result = await drive.updateDocument(documentWithIndices);

        expect(result).to.equals(0);
      });
    });
  });

  describe('#deleteDocument', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract);
    });

    context('without indices', () => {
      it('should should delete the document', async () => {
        // Create a document
        const documentWithoutIndices = documents[3];

        await drive.createDocument(documentWithoutIndices);

        const result = await drive.deleteDocument(
          dataContract,
          documentWithoutIndices.getType(),
          documentWithoutIndices.getId(),
        );

        expect(result).to.equals(0);
      });
    });

    context('with indices', () => {
      it('should should delete the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(documentWithIndices);

        const result = await drive.deleteDocument(
          dataContract,
          documentWithIndices.getType(),
          documentWithIndices.getId(),
        );

        expect(result).to.equals(0);
      });
    });
  });

  describe('#queryDocuments', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract);
    });

    it('should query existing documents', async () => {
      // TODO: Fix optional indexed field
      documents.pop();

      // Create documents
      await Promise.all(
        documents.map((document) => drive.createDocument(document)),
      );

      const fetchedDocuments = await drive.queryDocuments(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
        // orderBy: [['lastName', 'asc']], // TODO: We don't need orderBy for equal operator
      });

      expect(fetchedDocuments).to.have.lengthOf(1);
      expect(fetchedDocuments[0]).to.be.an.instanceOf(Document);
      expect(fetchedDocuments[0].toObject()).to.equal(documents[3].toObject());
    });

    it('should return empty array if documents are not exist', async () => {
      const fetchedDocuments = await drive.queryDocuments(dataContract, 'indexedDocument', {
        where: [['lastName', '==', 'Kennedy']],
      });

      expect(fetchedDocuments).to.have.lengthOf(0);
    });
  });
});
