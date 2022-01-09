const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const hash = require('@dashevo/dpp/lib/util/hash');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const wait = require('../../lib/wait');

xdescribe('Masternode Reward Shares', () => {
  let failed = false;
  let client;

  // Skip test if any prior test in this describe failed
  beforeEach(function beforeEach() {
    if (failed) {
      this.skip();
    }
  });

  afterEach(function afterEach() {
    failed = this.currentTest.state === 'failed';
  });

  before(async () => {
    client = await createClientWithFundedWallet();

    client.getApps().get('masternodeRewardShares').contractId = Identifier.from(
      process.env.MASTERNODE_REWARD_SHARES_CONTRACT_ID,
    );
  });

  after(async () => {
    if (client) {
      await client.disconnect();
    }
  });

  describe('Data Contract', () => {
    it('should exists', async () => {
      const createdDataContract = await client.platform.contracts.get(
        process.env.MASTERNODE_REWARD_SHARES_CONTRACT_ID,
      );

      expect(createdDataContract).to.exist();

      expect(createdDataContract.getId().toString()).to.equal(
        process.env.MASTERNODE_REWARD_SHARES_CONTRACT_ID,
      );
    });
  });

  describe.skip('Masternode owner', () => {
    let anotherIdentity;
    let rewardShare;
    let anotherRewardShare;
    let ownerIdentity;

    before(async function before() {
      if (!process.env.MASTERNODE_REWARD_SHARES_OWNER_PRIVATE_KEY
        || !process.env.MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH) {
        this.skip('masternode owner credentials are not set');
      }

      const ownerIdentifier = hash(
        Buffer.from(process.env.MASTERNODE_REWARD_SHARES_OWNER_PRO_REG_TX_HASH, 'hex'),
      );

      // Masternode identity should exist
      ownerIdentity = await client.platform.identities.get(ownerIdentifier);

      expect(ownerIdentity).to.exist();
    });

    it('should be able to create reward shares with existing identity', async () => {
      anotherIdentity = await client.platform.identities.register(5);

      rewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        ownerIdentity,
        {
          payToId: anotherIdentity.getId(),
          percentage: 1,
        },
      );

      client.platform.documents.broadcast({
        create: [rewardShare],
      });
    });

    it('should not be able to create reward shares with non-existing identity', async () => {
      const payToId = generateRandomIdentifier();

      const invalidRewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        ownerIdentity,
        {
          payToId,
          percentage: 1,
        },
      );

      try {
        await client.platform.documents.broadcast({
          create: [invalidRewardShare],
        }, ownerIdentity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal(`Identity ${payToId} doesn't exist`);
        expect(e.code).to.equal(4001);
      }
    });

    it('should be able to update reward shares with existing identity', async () => {
      rewardShare.set('percentage', 2);

      await client.platform.documents.broadcast({
        replace: [rewardShare],
      }, ownerIdentity);

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      const [updatedRewardShare] = await client.platform.documents.get('masternodeRewardShares.rewardShare', {
        where: [['$id', '==', rewardShare.getId()]],
      });

      expect(updatedRewardShare).to.exists();

      expect(updatedRewardShare.get('percentage')).equals(2);
    });

    it('should not be able to update reward shares with non-existing identity', async () => {
      const payToId = generateRandomIdentifier();

      rewardShare.set('payToId', payToId);

      try {
        await client.platform.documents.broadcast({
          replace: [rewardShare],
        }, ownerIdentity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal(`Identity ${payToId} doesn't exist`);
        expect(e.code).to.equal(4001);
      }
    });

    it('should not be able to share more than 100% of rewards', async () => {
      anotherIdentity = await client.platform.identities.register(5);

      anotherRewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        ownerIdentity,
        {
          payToId: anotherIdentity.getId(),
          percentage: 9999, // it will be 10001 in summary
        },
      );

      try {
        await client.platform.documents.broadcast({
          create: [rewardShare],
        }, ownerIdentity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Percentage can not be more than 10000');
        expect(e.code).to.equal(4001);
      }
    });

    it('should be able to remove reward shares', async () => {
      await client.platform.documents.broadcast({
        delete: [rewardShare, anotherRewardShare],
      }, ownerIdentity);
    });
  });

  describe.skip('Any Identity', () => {
    let identity;

    before(async () => {
      identity = await client.platform.identities.register(5);
    });

    it('should not be able to share rewards', async () => {
      const rewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        identity,
        {
          payToId: generateRandomIdentifier(),
          percentage: 1,
        },
      );

      try {
        await client.platform.documents.broadcast({
          create: [rewardShare],
        }, identity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Only masternode identities can share rewards');
        expect(e.code).to.equal(4001);
      }
    });
  });
});
