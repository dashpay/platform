const fs = require('fs');

const { expect } = require('chai');

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
      const result = await drive.applyContract(dataContract.toBuffer());

      expect(result).to.equals(0);
    });

    it('should update existing contract', async () => {
      await drive.applyContract(dataContract.toBuffer());

      dataContract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });

      const result = await drive.applyContract(dataContract.toBuffer());

      expect(result).to.equals(0);
    });
  });

  describe('documents', () => {
    beforeEach(async () => {
      await drive.createRootTree();

      await drive.applyContract(dataContract.toBuffer());
    });

    context('without indices', () => {
      it('should create a document', async () => {
        const documentWithoutIndices = documents[0];

        const result = await drive.createDocument(
          documentWithoutIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getOwnerId(),
          true,
        );

        expect(result).to.equals(0);
      });

      it('should should update a document', async () => {
        // Create a document
        const documentWithoutIndices = documents[0];

        await drive.createDocument(
          documentWithoutIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getOwnerId(),
          true,
        );

        // Update the document
        documentWithoutIndices.set('name', 'Bob');

        const result = await drive.updateDocument(
          documentWithoutIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getOwnerId(),
        );

        expect(result).to.equals(0);
      });

      it('should should delete the document', async () => {
        // Create a document
        const documentWithoutIndices = documents[3];

        await drive.createDocument(
          documentWithoutIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getOwnerId(),
          true,
        );

        const result = await drive.deleteDocument(
          documentWithoutIndices.getId(),
          dataContract.toBuffer(),
          documentWithoutIndices.getType(),
          documentWithoutIndices.getOwnerId(),
        );

        expect(result).to.equals(0);
      });
    });

    context('with indices', () => {
      it('should create a document', async () => {
        const documentWithIndices = documents[3];

        const result = await drive.createDocument(
          documentWithIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithIndices.getType(),
          documentWithIndices.getOwnerId(),
          true,
        );

        expect(result).to.equals(0);
      });

      it('should should update the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(
          documentWithIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithIndices.getType(),
          documentWithIndices.getOwnerId(),
          true,
        );

        // Update the document
        documentWithIndices.set('firstName', 'Bob');

        const result = await drive.updateDocument(
          documentWithIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithIndices.getType(),
          documentWithIndices.getOwnerId(),
        );

        expect(result).to.equals(0);
      });

      it('should should delete the document', async () => {
        // Create a document
        const documentWithIndices = documents[3];

        await drive.createDocument(
          documentWithIndices.toBuffer(),
          dataContract.toBuffer(),
          documentWithIndices.getType(),
          documentWithIndices.getOwnerId(),
          true,
        );

        const result = await drive.deleteDocument(
          documentWithIndices.getId(),
          dataContract.toBuffer(),
          documentWithIndices.getType(),
          documentWithIndices.getOwnerId(),
        );

        expect(result).to.equals(0);
      });
    });
  });
});
