const fs = require('fs');

const { expect } = require('chai');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const Drive = require('./index');

const TEST_DATA_PATH = './test_data';

describe('Drive', () => {
  let drive;

  beforeEach(() => {
    drive = new Drive(TEST_DATA_PATH);
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
    it('should create contract if not exists', async () => {
      await drive.createRootTree();

      const encodedContract = getDataContractFixture().toBuffer();

      const result = await drive.applyContract(encodedContract);

      expect(result).to.be.an.instanceOf(Number);
      expect(result).to.be.greaterThan(0);
    });

    it('should update existing contract', async () => {
      await drive.createRootTree();

      const contract = getDataContractFixture();

      let encodedContract = contract.toBuffer();

      await drive.applyContract(encodedContract);

      contract.setDocumentSchema('newDocumentType', {
        type: 'object',
        properties: {
          newProperty: {
            type: 'string',
          },
        },
        additionalProperties: false,
      });

      encodedContract = contract.toBuffer();

      const result = await drive.applyContract(encodedContract);

      expect(result).to.be.an.instanceOf(Number);
      expect(result).to.be.greaterThan(0);
    });
  });
});
