const {expect} = require('chai');
const Dash = require('../../dist/dash.cjs.min.js');
const fixtures = require('../fixtures/user-flow-1');
const Chance = require('chance');
const chance = new Chance();

let clientInstance;
let hasBalance=false;
let hasDuplicate=true;
let createdIdentityId;
let createdIdentity;

const year = chance.birthday({string: true}).slice(-2);
const firstname = chance.first();
const username = `test-${firstname}${year}`;

const clientOpts = {
  network: fixtures.network,
  mnemonic: fixtures.mnemonic
};
describe('Integration - User flow 1 - Identity, DPNS, Documents', function suite() {
  this.timeout(240000);

  it('should init a Client', async () => {
    clientInstance = new Dash.Client(clientOpts);
    expect(clientInstance.network).to.equal('testnet');
    expect(clientInstance.accountIndex).to.equal(0);
    expect(clientInstance.apps).to.deep.equal({dpns: {contractId: "77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3"}});
    expect(clientInstance.wallet.network).to.equal('testnet');
    expect(clientInstance.wallet.offlineMode).to.equal(false);
    expect(clientInstance.wallet.mnemonic).to.equal(fixtures.mnemonic);
    expect(clientInstance.wallet.walletId).to.equal('6afaad2189');
    expect(clientInstance.account.index).to.equal(0);
    expect(clientInstance.account.walletId).to.equal('6afaad2189');
    expect(clientInstance.account.getUnusedAddress().address).to.equal('yj8sq7ogzz6JtaxpBQm5Hg9YaB5cKExn5T');

    expect(clientInstance.platform.dpp).to.exist;
    expect(clientInstance.platform.client).to.exist;
  });
  it('should be ready quickly', (done) => {
    let timer = setTimeout(() => {
      done(new Error('Should have been initialized in time'));
    }, 15000);
    clientInstance.isReady().then(() => {
      clearTimeout(timer);
      expect(clientInstance.account.state).to.deep.equal({isInitialized: true, isReady: true, isDisconnecting: false});
      expect(clientInstance.state).to.deep.equal({isReady: true, isAccountReady: true});
      expect(clientInstance.apps['dpns']).to.exist;
      expect(clientInstance.apps['dpns'].contractId).to.equal('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3');
      expect(clientInstance.apps['dpns'].contractId).to.equal('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3');
      return done();
    })
  });
  it('should have a balance', function (done) {
    const balance = (clientInstance.account.getTotalBalance());
    if(balance<10000){
      return done(new Error(`You need to fund this address : ${clientInstance.account.getUnusedAddress().address}. Insuffisiant balance: ${balance}`));
    }
    hasBalance = true;
    return done();
  });
  it('should check it\'s DPNS reg is available' , async function () {
    const getDocument = await clientInstance.platform.names.get(username);
    expect(getDocument).to.equal(null);
    hasDuplicate = false;
  });
  it('should register an identity', async function () {
    if(!hasBalance){
      throw new Error('Insufficient balance to perform this test')
    }
    createdIdentityId = await clientInstance.platform.identities.register();
    expect(createdIdentityId).to.not.equal(null);
    expect(createdIdentityId.length).to.gte(42);
    expect(createdIdentityId.length).to.lte(44);
  });

  it('should fetch the identity back', async function () {
    if(!createdIdentityId){
      throw new Error('Can\'t perform the test. Failed to create identity');
    }
    const fetchIdentity = await clientInstance.platform.identities.get(createdIdentityId);

    expect(fetchIdentity).to.exist;
    expect(fetchIdentity.id).to.equal(createdIdentityId);
    expect(fetchIdentity.type).to.equal(1);
    createdIdentity = fetchIdentity;
  });
  it('should register a name', async function () {
    if(!createdIdentity){
      throw new Error('Can\'t perform the test. Failed to fetch identity');
    }
    if(hasDuplicate){
      throw new Error(`Duplicate username ${username} registered. Skipping.`)
    }
    const createDocument = await clientInstance.platform.names.register(username, createdIdentity);
    expect(createDocument.action).to.equal(1);
    expect(createDocument.type).to.equal('domain');
    expect(createDocument.userId).to.equal(createdIdentityId);
    expect(createDocument.contractId).to.equal('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3');
    expect(createDocument.data.label).to.equal(username)
    expect(createDocument.data.normalizedParentDomainName).to.equal('dash');
  });

  it('should retrieve itself by document', async function () {
    if(!createdIdentity){
      throw new Error('Can\'t perform the test. Failed to fetch identity & did not reg name');
    }
    const [doc] = await clientInstance.platform.documents.get('dpns.domain', {where:[
        ["normalizedParentDomainName","==","dash"],
        ["normalizedLabel","==",username.toLowerCase()],
      ]})
    expect(doc.revision).to.equal(1);
    expect(doc.type).to.equal('domain');
    expect(doc.userId).to.equal(createdIdentityId);
    expect(doc.contractId).to.equal('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3');
    expect(doc.data.label).to.equal(username)
    expect(doc.data.normalizedParentDomainName).to.equal('dash');
  });
  it('should disconnect', async function () {
    await clientInstance.disconnect();
  });
});

