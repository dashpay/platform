const generateDapObjectId = require('../../../../lib/stateView/dapObject/generateDapObjectId');

describe('generateDapObjectId', () => {
  it('should generate DAP Object ID from Blockchain User Id and Slot number', async () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const slotNumber = 0;

    const id = generateDapObjectId(blockchainUserId, slotNumber);

    expect(id).to.be.equal('9w7EpSsijgddB9M7QzcHiwykcMDTRzm4t2txm3ge7pQd');
  });
});
