const fs = require('fs');

const { expect } = require('chai');

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
});
