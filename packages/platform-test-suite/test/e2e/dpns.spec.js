const crypto = require('crypto');

const {
  contractId: dpnsContractId,
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

    it('should not be able to register TLD', async () => {
      let broadcastError;

      try {
        await client.platform.names.register(getRandomDomain(), {
          identity: identity.getId(),
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Can\'t create top level domain for this identity');
      expect(broadcastError.code).to.equal(40500);
    });

    it('should be able to register a second level domain', async () => {
      registeredDomain = await client.platform.names.register(`${secondLevelDomain}0.${topLevelDomain}`, {
        identity: identity.getId(),
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      expect(registeredDomain.getType()).to.equal('domain');
      expect(registeredDomain.getData().label).to.equal(`${secondLevelDomain}0`);
      expect(registeredDomain.getData().normalizedParentDomainName).to.equal(topLevelDomain);
    });

    it('should not be able register similar domain name', async () => {
      let broadcastError;

      try {
        const domain = `${secondLevelDomain}O.${topLevelDomain}`;

        await client.platform.names.register(domain, {
          identity: identity.getId(),
        }, identity);

        expect.fail('should throw error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.be.equal(40105);
      expect(broadcastError.message).to.match(/Document \w* has duplicate unique properties \["normalizedParentDomainName", "normalizedLabel"] with other documents/);
    });

    it('should not be able to register a subdomain for parent domain which is not exist', async () => {
      let broadcastError;

      try {
        const domain = `${getRandomDomain()}.${getRandomDomain()}`;

        await client.platform.names.register(domain, {
          identity: identity.getId(),
        }, identity);

        expect.fail('should throw error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Parent domain is not present');
      expect(broadcastError.code).to.equal(40500);
    });

    it('should be able to search a domain', async () => {
      const documents = await client.platform.names.search(secondLevelDomain, topLevelDomain);

      expect(documents).to.have.lengthOf(1);

      const rawDocument = documents[0].toObject();

      delete rawDocument.$createdAt;
      delete rawDocument.$createdAtCoreBlockHeight;
      delete rawDocument.$createdAtBlockHeight;
      delete rawDocument.$updatedAt;
      delete rawDocument.$updatedAtCoreBlockHeight;
      delete rawDocument.$updatedAtBlockHeight;
      delete rawDocument.$transferredAt;
      delete rawDocument.$transferredAtCoreBlockHeight;
      delete rawDocument.$transferredAtBlockHeight;
      delete rawDocument.preorderSalt;

      const rawRegisteredDomain = registeredDomain.toObject();

      delete rawRegisteredDomain.$createdAt;
      delete rawRegisteredDomain.$createdAtCoreBlockHeight;
      delete rawRegisteredDomain.$createdAtBlockHeight;
      delete rawRegisteredDomain.$updatedAt;
      delete rawRegisteredDomain.$updatedAtCoreBlockHeight;
      delete rawRegisteredDomain.$updatedAtBlockHeight;
      delete rawRegisteredDomain.$transferredAt;
      delete rawRegisteredDomain.$transferredAtCoreBlockHeight;
      delete rawRegisteredDomain.$transferredAtBlockHeight;
      delete rawRegisteredDomain.preorderSalt;

      expect(rawDocument).to.deep.equal(rawRegisteredDomain);
    });

    it('should be able to resolve domain by it\'s name', async () => {
      const document = await client.platform.names.resolve(`${secondLevelDomain}0.${topLevelDomain}`);

      const rawDocument = document.toObject();

      delete rawDocument.$createdAt;
      delete rawDocument.$createdAtCoreBlockHeight;
      delete rawDocument.$createdAtBlockHeight;
      delete rawDocument.$updatedAt;
      delete rawDocument.$updatedAtCoreBlockHeight;
      delete rawDocument.$updatedAtBlockHeight;
      delete rawDocument.$transferredAt;
      delete rawDocument.$transferredAtCoreBlockHeight;
      delete rawDocument.$transferredAtBlockHeight;
      delete rawDocument.preorderSalt;

      const rawRegisteredDomain = registeredDomain.toObject();

      delete rawRegisteredDomain.$createdAt;
      delete rawRegisteredDomain.$createdAtCoreBlockHeight;
      delete rawRegisteredDomain.$createdAtBlockHeight;
      delete rawRegisteredDomain.$updatedAt;
      delete rawRegisteredDomain.$updatedAtCoreBlockHeight;
      delete rawRegisteredDomain.$updatedAtBlockHeight;
      delete rawRegisteredDomain.$transferredAt;
      delete rawRegisteredDomain.$transferredAtCoreBlockHeight;
      delete rawRegisteredDomain.$transferredAtBlockHeight;
      delete rawRegisteredDomain.preorderSalt;

      expect(rawDocument).to.deep.equal(rawRegisteredDomain);
    });

    it('should be able to resolve domain by it\'s record', async () => {
      const [document] = await client.platform.names.resolveByRecord(
        'identity',
        registeredDomain.getData().records.identity,
      );

      const rawDocument = document.toObject();

      delete rawDocument.$createdAt;
      delete rawDocument.$createdAtCoreBlockHeight;
      delete rawDocument.$createdAtBlockHeight;
      delete rawDocument.$updatedAt;
      delete rawDocument.$updatedAtCoreBlockHeight;
      delete rawDocument.$updatedAtBlockHeight;
      delete rawDocument.$transferredAt;
      delete rawDocument.$transferredAtCoreBlockHeight;
      delete rawDocument.$transferredAtBlockHeight;
      delete rawDocument.preorderSalt;

      const rawRegisteredDomain = registeredDomain.toObject();

      delete rawRegisteredDomain.$createdAt;
      delete rawRegisteredDomain.$createdAtCoreBlockHeight;
      delete rawRegisteredDomain.$createdAtBlockHeight;
      delete rawRegisteredDomain.$updatedAt;
      delete rawRegisteredDomain.$updatedAtCoreBlockHeight;
      delete rawRegisteredDomain.$updatedAtBlockHeight;
      delete rawRegisteredDomain.$transferredAt;
      delete rawRegisteredDomain.$transferredAtCoreBlockHeight;
      delete rawRegisteredDomain.$transferredAtBlockHeight;
      delete rawRegisteredDomain.preorderSalt;

      expect(rawDocument).to.deep.equal(rawRegisteredDomain);
    });

    // TODO: Enable test when we figure out how to skip a check in the SDK's state transition
    //  factory
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
      expect(broadcastError.code).to.equal(40500);
    });

    // TODO: Enable test when we documentsMutable true fixed and do not prevent from deleting
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
      expect(broadcastError.code).to.equal(40500);
    });

    it('should not be able to register two domains with same `identity` record');

    it('should be able to register many domains with same `identity` record');

    it('should not be able to update preorder');

    it('should not be able to domain preorder');
  });
});
