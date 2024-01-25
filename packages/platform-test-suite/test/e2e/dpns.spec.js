const crypto = require('crypto');

const {
  contractId: dpnsContractId,
  ownerId: dpnsOwnerId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');

const getRandomDomain = () => crypto.randomBytes(10).toString('hex');

describe('DPNS', () => {
  let failed = false;
  let client;
  let identity;
  let topLevelDomain;
  let secondLevelDomain;
  let registeredDomain;

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
    topLevelDomain = 'dash';
    secondLevelDomain = getRandomDomain();
    client = await createClientWithFundedWallet(1000000);

    await client.platform.identities.topUp(dpnsOwnerId, 300000);
  });

  after(async () => {
    if (client) {
      await client.disconnect();
    }
  });

  describe('Data contract', () => {
    it('should exists', async () => {
      const createdDataContract = await client.platform.contracts.get(dpnsContractId);

      expect(createdDataContract).to.exist();
      expect(createdDataContract.getId().toString()).to.equal(dpnsContractId);
    });
  });

  describe('DPNS owner', () => {
    let createdTLD;
    let newTopLevelDomain;
    let ownerClient;

    before(async () => {
      ownerClient = await createClientWithFundedWallet(
        10000,
        process.env.DPNS_OWNER_PRIVATE_KEY,
      );

      newTopLevelDomain = getRandomDomain();
      identity = await ownerClient.platform.identities.get(dpnsOwnerId);

      expect(identity).to.exist();
      await ownerClient.platform.identities.topUp(dpnsOwnerId, 1000);
    });

    after(async () => {
      if (ownerClient) {
        await ownerClient.disconnect();
      }
    });

    // generate a random one which will be used in tests above
    // skip if DPNS owner private key is not passed and use `dash` in tests above
    it('should be able to register a TLD', async () => {
      createdTLD = await ownerClient.platform.names.register(newTopLevelDomain, {
        dashAliasIdentityId: identity.getId(),
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      expect(createdTLD).to.exist();
      expect(createdTLD.getType()).to.equal('domain');
      expect(createdTLD.getData().label).to.equal(newTopLevelDomain);
      expect(createdTLD.getData().normalizedParentDomainName).to.equal('');
    });

    // TODO(rs-drive-abci): restore
    //   There are two DataTriggerAction errors now. One is used in rs-drive-abci data trigger (new)
    //   And other is in dpp data trigger (old). New error is dispatched in this test,
    //   but it does not have binding yet.
    //   Decide what to do with rs-dpp data trigger and it's errors,
    //   and update bindings and test accordingly
    it.skip('should not be able to update domain', async () => {
      createdTLD.set('label', 'anotherlabel');

      let broadcastError;

      try {
        await ownerClient.platform.documents.broadcast({
          replace: [createdTLD],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Action is not allowed');
      expect(broadcastError.code).to.equal(4001);
    });

    // TODO(rs-drive-abci): restore
    //   There are two DataTriggerAction errors now. One is used in rs-drive-abci data trigger (new)
    //   And other is in dpp data trigger (old). New error is dispatched in this test,
    //   but it does not have binding yet.
    //   Decide what to do with rs-dpp data trigger and it's errors,
    //   and update bindings and test accordingly
    it.skip('should not be able to delete domain', async () => {
      let broadcastError;

      try {
        await ownerClient.platform.documents.broadcast({
          delete: [createdTLD],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Action is not allowed');
      expect(broadcastError.code).to.equal(4001);
    });
  });

  describe('Any Identity', () => {
    before(async () => {
      identity = await client.platform.identities.register(600000);

      await waitForSTPropagated();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    // TODO(rs-drive-abci): test randomly returns StateTransition already in chain error,
    //   but it's happening because of retry attempts for the same ST.
    //   Underlying issue causing retry is different and should be debugged.
    //   (console.log error in dapi-client's GrpcTransport for more details)
    it.skip('should not be able to register TLD', async () => {
      let broadcastError;

      try {
        await client.platform.names.register(getRandomDomain(), {
          dashAliasIdentityId: identity.getId(),
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Can\'t create top level domain for this identity');
      expect(broadcastError.code).to.equal(4001);
    });

    // TODO(rs-drive-abci): test randomly returns StateTransition already in chain error,
    //   but it's happening because of retry attempts for the same ST.
    //   Underlying issue causing retry is different and should be debugged.
    //   (console.log error in dapi-client's GrpcTransport for more details)
    it.skip('should be able to register a second level domain', async () => {
      registeredDomain = await client.platform.names.register(`${secondLevelDomain}0.${topLevelDomain}`, {
        dashUniqueIdentityId: identity.getId(),
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      expect(registeredDomain.getType()).to.equal('domain');
      expect(registeredDomain.getData().label).to.equal(secondLevelDomain);
      expect(registeredDomain.getData().normalizedParentDomainName).to.equal(topLevelDomain);
    });

    it.skip('should not be able register similar domain name', async () => {
      let broadcastError;

      try {
        const domain = `${secondLevelDomain}O.${topLevelDomain}`;

        await client.platform.names.register(domain, {
          dashAliasIdentityId: identity.getId(),
        }, identity);

        expect.fail('should throw error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.be.equal(4009);
      expect(broadcastError.message).to.match(/Document \w* has duplicate unique properties \["normalizedLabel", "normalizedParentDomainName"] with other documents/);
    });

    // TODO(rs-drive-abci): test randomly returns StateTransition already in chain error,
    //   but it's happening because of retry attempts for the same ST.
    //   Underlying issue causing retry is different and should be debugged.
    //   (console.log error in dapi-client's GrpcTransport for more details)
    it.skip('should not be able to register a subdomain for parent domain which is not exist', async () => {
      let broadcastError;

      try {
        const domain = `${getRandomDomain()}.${getRandomDomain()}.${topLevelDomain}`;

        await client.platform.names.register(domain, {
          dashAliasIdentityId: identity.getId(),
        }, identity);

        expect.fail('should throw error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Parent domain is not present');
      expect(broadcastError.code).to.equal(4001);
    });

    // TODO(rs-drive-abci): restore - toObject mismatch (happens randomly)
    it.skip('should be able to search a domain', async () => {
      const documents = await client.platform.names.search(secondLevelDomain, topLevelDomain);

      expect(documents).to.have.lengthOf(1);

      const [document] = documents;

      expect(document.toObject()).to.deep.equal(registeredDomain.toObject());
    });

    // TODO(rs-drive-abci): restore - toObject mismatch (happens randomly)
    it.skip('should be able to resolve domain by it\'s name', async () => {
      const document = await client.platform.names.resolve(`${secondLevelDomain}.${topLevelDomain}`);

      expect(document.toObject()).to.deep.equal(registeredDomain.toObject());
    });

    // TODO(rs-drive-abci): restore - toObject mismatch (happens randomly)
    it.skip('should be able to resolve domain by it\'s record', async () => {
      const [document] = await client.platform.names.resolveByRecord(
        'dashUniqueIdentityId',
        registeredDomain.getData().records.dashUniqueIdentityId,
      );

      expect(document.toObject()).to.deep.equal(registeredDomain.toObject());
    });

    // TODO(rs-drive-abci): restore
    //   There are two DataTriggerAction errors now. One is used in rs-drive-abci data trigger (new)
    //   And other is in dpp data trigger (old). New error is dispatched in this test,
    //   but it does not have binding yet.
    //   Decide what to do with rs-dpp data trigger and it's errors,
    //   and update bindings and test accordingly
    it.skip('should not be able to update domain', async () => {
      registeredDomain.set('label', 'newlabel');

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          replace: [registeredDomain],
        }, identity);

        expect.fail('should throw an error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Action is not allowed');
      expect(broadcastError.code).to.equal(4001);
    });

    // TODO(rs-drive-abci): restore
    //   There are two DataTriggerAction errors now. One is used in rs-drive-abci data trigger (new)
    //   And other is in dpp data trigger (old). New error is dispatched in this test,
    //   but it does not have binding yet.
    //   Decide what to do with rs-dpp data trigger and it's errors,
    //   and update bindings and test accordingly
    it.skip('should not be able to delete domain', async () => {
      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          delete: [registeredDomain],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Action is not allowed');
      expect(broadcastError.code).to.equal(4001);
    });

    it('should not be able to register two domains with same `dashAliasIdentityId` record');

    it('should be able to register many domains with same `dashAliasIdentityId` record');

    it('should not be able to update preorder');

    it('should not be able to domain preorder');
  });
});
