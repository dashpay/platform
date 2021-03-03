const crypto = require('crypto');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');

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
    client = await createClientWithFundedWallet();
  });

  after(async () => {
    await client.disconnect();
  });

  describe('Data contract', () => {
    it('should exists', async () => {
      const createdDataContract = await client.platform.contracts.get(process.env.DPNS_CONTRACT_ID);

      expect(createdDataContract).to.exist();
      expect(createdDataContract.getId().toString()).to.equal(process.env.DPNS_CONTRACT_ID);
    });
  });

  describe('DPNS owner', () => {
    let createdTLD;
    let newTopLevelDomain;
    let ownerClient;

    before(async () => {
      ownerClient = await createClientWithFundedWallet(
        process.env.DPNS_TOP_LEVEL_IDENTITY_PRIVATE_KEY,
      );

      newTopLevelDomain = getRandomDomain();
      identity = await ownerClient.platform.identities.get(process.env.DPNS_TOP_LEVEL_IDENTITY_ID);

      expect(identity).to.exist();
      await ownerClient.platform.identities.topUp(process.env.DPNS_TOP_LEVEL_IDENTITY_ID, 5);
    });

    after(async () => {
      await ownerClient.disconnect();
    });

    // generate a random one which will be used in tests above
    // skip if DPNS owner private key is not passed and use `dash` in tests above
    it('should be able to register a TLD', async () => {
      createdTLD = await ownerClient.platform.names.register(newTopLevelDomain, {
        dashAliasIdentityId: identity.getId(),
      }, identity);

      expect(createdTLD).to.exist();
      expect(createdTLD.getType()).to.equal('domain');
      expect(createdTLD.getData().label).to.equal(newTopLevelDomain);
      expect(createdTLD.getData().normalizedParentDomainName).to.equal('');
    });

    it('should not be able to update domain', async () => {
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
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Action is not allowed');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Action is not allowed');
    });

    it('should not be able to delete domain', async () => {
      let broadcastError;

      try {
        await ownerClient.platform.documents.broadcast({
          delete: [createdTLD],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Action is not allowed');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Action is not allowed');
    });
  });

  describe('Any Identity', () => {
    before(async () => {
      identity = await client.platform.identities.register(5);
    });

    after(async () => {
      await client.disconnect();
    });

    it('should not be able to register TLD', async () => {
      let broadcastError;

      try {
        await client.platform.names.register(getRandomDomain(), {
          dashAliasIdentityId: identity.getId(),
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Can\'t create top level domain for this identity');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Can\'t create top level domain for this identity');
    });

    it('should be able to register a second level domain', async () => {
      registeredDomain = await client.platform.names.register(`${secondLevelDomain}.${topLevelDomain}`, {
        dashUniqueIdentityId: identity.getId(),
      }, identity);

      expect(registeredDomain.getType()).to.equal('domain');
      expect(registeredDomain.getData().label).to.equal(secondLevelDomain);
      expect(registeredDomain.getData().normalizedParentDomainName).to.equal(topLevelDomain);
    });

    it('should not be able to register a subdomain for parent domain which is not exist', async () => {
      let broadcastError;

      try {
        const domain = `${getRandomDomain()}.${getRandomDomain()}.${topLevelDomain}`;

        await client.platform.names.register(domain, {
          dashAliasIdentityId: identity.getId(),
        }, identity);

        expect.fail('Should throw error');
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Parent domain is not present');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Parent domain is not present');
    });

    it('should be able to search a domain', async () => {
      const documents = await client.platform.names.search(secondLevelDomain, topLevelDomain);

      expect(documents).to.have.lengthOf(1);

      const [document] = documents;

      expect(document.toJSON()).to.deep.equal(registeredDomain.toJSON());
    });

    it('should be able to resolve domain by it\'s name', async () => {
      const document = await client.platform.names.resolve(`${secondLevelDomain}.${topLevelDomain}`);

      expect(document.toJSON()).to.deep.equal(registeredDomain.toJSON());
    });

    it('should be able to resolve domain by it\'s record', async () => {
      const [document] = await client.platform.names.resolveByRecord(
        'dashUniqueIdentityId',
        registeredDomain.getData().records.dashUniqueIdentityId,
      );

      expect(document.toJSON()).to.deep.equal(registeredDomain.toJSON());
    });

    it('should not be able to update domain', async () => {
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
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Action is not allowed');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Action is not allowed');
    });

    it('should not be able to delete domain', async () => {
      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          delete: [registeredDomain],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DataTriggerConditionError: Action is not allowed');
      expect(broadcastError.data.errors[0].name).to.equal('DataTriggerConditionError');
      expect(broadcastError.data.errors[0].message).to.equal('Action is not allowed');
    });

    it('should not be able to register two domains with same `dashAliasIdentityId` record');

    it('should be able to register many domains with same `dashAliasIdentityId` record');

    it('should not be able to update preorder');

    it('should not be able to domain preorder');
  });
});
