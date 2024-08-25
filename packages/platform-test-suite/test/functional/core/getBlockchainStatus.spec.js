const { Essentials: { Buffer } } = require('dash');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getBlockchainStatus', function main() {
    let client;

    this.timeout(160000);

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should return status', async () => {
      const result = await client.getDAPIClient().core.getBlockchainStatus();

      const {
        version, time, status, syncProgress, chain, network,
      } = result;

      expect(version.protocol).to.be.a('number');
      expect(version.software).to.be.a('number');
      expect(version.agent).to.be.a('string');

      expect(time.now).to.be.a('number');
      expect(time.offset).to.be.a('number');
      expect(time.median).to.be.a('number');

      expect(status).to.be.a('string');

      expect(syncProgress).to.be.a('number');

      expect(chain.name).to.be.a('string');
      expect(chain.headersCount).to.be.a('number');
      expect(chain.blocksCount).to.be.a('number');
      expect(chain.bestBlockHash).to.be.an.instanceOf(Buffer);
      expect(chain.difficulty).to.be.a('number');
      expect(chain.chainWork).to.be.an.instanceOf(Buffer);
      expect(chain.isSynced).to.be.a('boolean');
      expect(chain.syncProgress).to.be.a('number');

      expect(network.peersCount).to.be.a('number');
      expect(network.fee.relay).to.be.a('number');
      expect(network.fee.incremental).to.be.a('number');
    });
  });
});
