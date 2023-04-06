const Dash = require('dash');

const {
  contractId: masternodeRewardSharesContractId,
} = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');

const generateRandomIdentifier = require('../../lib/test/utils/generateRandomIdentifier');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');

const { PlatformProtocol: { IdentityPublicKey } } = Dash;

describe('Masternode Reward Shares', () => {
  let failed = false;
  let client;
  // let dpp;

  before(async () => {
    // dpp = new Dash.PlatformProtocol();
    // await dpp.initialize();

    client = await createClientWithFundedWallet(
      8000000,
    );

    await client.platform.initialize();

    const masternodeRewardSharesContract = await client.platform.contracts.get(
      masternodeRewardSharesContractId,
    );

    client.getApps().set('masternodeRewardShares', {
      contractId: masternodeRewardSharesContractId,
      contract: masternodeRewardSharesContract,
    });
  });

  // Skip test if any prior test in this describe failed
  beforeEach(function beforeEach() {
    if (failed) {
      this.skip();
    }
  });

  afterEach(function afterEach() {
    failed = this.currentTest.state === 'failed';
  });

  after(async () => {
    if (client) {
      await client.disconnect();
    }
  });

  describe('Data Contract', () => {
    it('should exists', async () => {
      const createdDataContract = await client.platform.contracts.get(
        masternodeRewardSharesContractId,
      );

      expect(createdDataContract).to.exist();

      expect(createdDataContract.getId().toString()).to.equal(
        masternodeRewardSharesContractId,
      );
    });
  });

  describe('Masternode owner', () => {
    let anotherIdentity;
    let rewardShare;
    let anotherRewardShare;
    let masternodeOwnerMasterPrivateKey;
    let masternodeOwnerIdentity;
    let derivedPrivateKey;
    let signaturePublicKeyId;

    before(async function before() {
      if (!process.env.MASTERNODE_OWNER_PRO_REG_TX_HASH
        || !process.env.MASTERNODE_OWNER_MASTER_PRIVATE_KEY) {
        this.skip('masternode owner credentials are not set');
      }

      const masternodeOwnerIdentifier = Buffer.from(process.env.MASTERNODE_OWNER_PRO_REG_TX_HASH, 'hex');

      masternodeOwnerIdentity = await client.platform.identities.get(masternodeOwnerIdentifier);

      masternodeOwnerMasterPrivateKey = process.env.MASTERNODE_OWNER_MASTER_PRIVATE_KEY;

      // Masternode identity should exist
      expect(masternodeOwnerIdentity).to.exist();

      await client.platform.identities.topUp(masternodeOwnerIdentity.getId(), 1900000);

      // Since we cannot create "High" level key for masternode Identities automatically,
      // (this key is used to sign state transitions, other than "update")
      // we add this key here

      signaturePublicKeyId = masternodeOwnerIdentity.getPublicKeyMaxId() + 1;

      // Get Masternode Rewards Share Contract owner account
      const account = await client.platform.client.getWalletAccount();

      ({ privateKey: derivedPrivateKey } = account
        .identities
        .getIdentityHDKeyByIndex(
          1000,
          signaturePublicKeyId,
        ));

      const { IdentityPublicKeyWithWitness } = client.platform.dppModule;

      const identityPublicKey = derivedPrivateKey.toPublicKey().toBuffer();

      const newPublicKey = new IdentityPublicKeyWithWitness(
        {
          id: signaturePublicKeyId,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
          data: identityPublicKey,
          readOnly: false,
          signature: Buffer.alloc(0),
        },
      );

      const update = {
        add: [newPublicKey],
      };

      const stateTransition = client.platform.wasmDpp.identity.createIdentityUpdateTransition(
        masternodeOwnerIdentity,
        update,
      );

      const signerKey = masternodeOwnerIdentity.getPublicKeys()[0];

      const promises = stateTransition.getPublicKeysToAdd().map(async (publicKey) => {
        stateTransition.setSignaturePublicKeyId(signerKey.getId());

        await stateTransition.signByPrivateKey(derivedPrivateKey.toBuffer(), publicKey.getType());

        publicKey.setSignature(stateTransition.getSignature());

        stateTransition.setSignature(undefined);
        stateTransition.setSignaturePublicKeyId(undefined);
      });

      await Promise.all(promises);

      stateTransition.setSignaturePublicKeyId(0);

      await stateTransition.signByPrivateKey(
        masternodeOwnerMasterPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      await client.platform.broadcastStateTransition(
        stateTransition,
      );
    });

    it('should be able to create reward shares with existing identity', async () => {
      anotherIdentity = await client.platform.identities.register(100000);

      rewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        masternodeOwnerIdentity,
        {
          payToId: anotherIdentity.getId(),
          percentage: 1,
        },
      );

      const stateTransition = client.platform.document.createStateTransition({
        create: [rewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      await client.platform.broadcastStateTransition(
        stateTransition,
      );
    });

    it('should not be able to create reward shares with non-existing identity', async () => {
      const payToId = generateRandomIdentifier();

      const invalidRewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        masternodeOwnerIdentity,
        {
          payToId,
          percentage: 1,
        },
      );

      const stateTransition = dpp.document.createStateTransition({
        create: [invalidRewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      try {
        await client.platform.broadcastStateTransition(
          stateTransition,
        );

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal(`Identity ${payToId} doesn't exist`);
        expect(e.code).to.equal(4001);
      }
    });

    it('should be able to update reward shares with existing identity', async () => {
      rewardShare.set('percentage', 2);

      const stateTransition = dpp.document.createStateTransition({
        replace: [rewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      await client.platform.broadcastStateTransition(
        stateTransition,
      );

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const [updatedRewardShare] = await client.platform.documents.get('masternodeRewardShares.rewardShare', {
        where: [['$id', '==', rewardShare.getId()]],
      });

      expect(updatedRewardShare).to.exists();

      expect(updatedRewardShare.get('percentage')).equals(2);
    });

    it('should not be able to update reward shares with non-existing identity', async () => {
      const payToId = generateRandomIdentifier();

      [rewardShare] = await client.platform.documents.get(
        'masternodeRewardShares.rewardShare',
        { where: [['$id', '==', rewardShare.getId()]] },
      );

      rewardShare.set('payToId', payToId);

      const stateTransition = dpp.document.createStateTransition({
        replace: [rewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      try {
        await client.platform.broadcastStateTransition(
          stateTransition,
        );

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal(`Identity ${payToId} doesn't exist`);
        expect(e.code).to.equal(4001);
      }
    });

    it('should not be able to share more than 100% of rewards', async () => {
      anotherIdentity = await client.platform.identities.register(100000);

      await waitForSTPropagated();

      anotherRewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        masternodeOwnerIdentity,
        {
          payToId: anotherIdentity.getId(),
          percentage: 9999, // it will be 10001 in summary
        },
      );

      const stateTransition = dpp.document.createStateTransition({
        create: [anotherRewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      try {
        await client.platform.broadcastStateTransition(
          stateTransition,
        );

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Percentage can not be more than 10000');
        expect(e.code).to.equal(4001);
      }
    });

    it('should be able to remove reward shares', async () => {
      const stateTransition = dpp.document.createStateTransition({
        delete: [rewardShare],
      });

      stateTransition.setSignaturePublicKeyId(signaturePublicKeyId);

      await stateTransition.signByPrivateKey(
        derivedPrivateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      await client.platform.broadcastStateTransition(
        stateTransition,
      );

      await waitForSTPropagated();

      const [storedDocument] = await client.platform.documents.get(
        'masternodeRewardShares.rewardShare',
        { where: [['$id', '==', rewardShare.getId()]] },
      );

      expect(storedDocument).to.not.exist();
    });
  });

  describe('Any Identity', () => {
    let identity;

    before(async () => {
      identity = await client.platform.identities.register(200000);

      await waitForSTPropagated();
    });

    it('should not be able to share rewards', async () => {
      const rewardShare = await client.platform.documents.create(
        'masternodeRewardShares.rewardShare',
        identity,
        {
          payToId: await generateRandomIdentifier(),
          percentage: 1,
        },
      );
      const stateTransition = client.platform.wasmDpp.document.createStateTransition({
        create: [rewardShare],
      });

      stateTransition.setSignaturePublicKey(1);

      const account = await client.getWalletAccount();

      const { privateKey } = account.identities.getIdentityHDKeyById(
        identity.getId().toString(),
        1,
      );

      await stateTransition.signByPrivateKey(
        privateKey,
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
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
