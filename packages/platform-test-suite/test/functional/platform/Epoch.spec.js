const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Platform', () => {
  describe('Epoch', () => {
    let client;

    before(async () => {
      client = await createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    describe('getEpochsInfo', () => {
      it('should return epoch information', async () => {
        const response = await client.dapiClient.platform.getEpochsInfo(0, 1);

        expect(response.getEpochsInfo()).to.be.an('array');
        expect(response.getEpochsInfo()).to.have.lengthOf(1);

        const genesisEpoch = response.getEpochsInfo()[0];

        expect(genesisEpoch.number).to.equal(0);
        expect(genesisEpoch.firstBlockHeight).to.equal(BigInt(1));
      });
    });
  });
});
