const {expect} = require('chai');
const Dash = require('../../dist/dash.cjs.min.js');
const fixtures = require('../fixtures/user-flow-1');
const Chance = require('chance');
const chance = new Chance();

const dotenvSafe = require('dotenv-safe');

const path = require('path');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

const {
  Transaction,
  PrivateKey
} = require('@dashevo/dashcore-lib');

function wait(ms) {
  return new Promise((res) => setTimeout(res, ms));
}

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {Address} faucetAddress
 * @param {PrivateKey} faucetPrivateKey
 * @param {Address} address
 * @param {number} amount
 * @return {Promise<string>}
 */
async function fundAddress(dapiClient, faucetAddress, faucetPrivateKey, address, amount) {
  const { items: inputs } = await dapiClient.getUTXO(faucetAddress);

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .to(address, amount)
    .change(faucetAddress)
    .fee(668)
    .sign(faucetPrivateKey);

  let { blocks: currentBlockHeight } = await dapiClient.getStatus();

  const transactionId = await dapiClient.sendTransaction(transaction.toBuffer());

  const desiredBlockHeight = currentBlockHeight + 2;

  do {
    ({ blocks: currentBlockHeight } = await dapiClient.getStatus());
    await wait(30000);
  } while (currentBlockHeight < desiredBlockHeight);

  return transactionId;
}


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
  wallet: {
    mnemonic: null,
  },
};
describe('Integration - User flow 1 - Identity, DPNS, Documents', function suite() {
  this.timeout(450000);

  it('should init a Client', async () => {
    clientInstance = new Dash.Client(clientOpts);
    expect(clientInstance.network).to.equal('testnet');
    expect(clientInstance.accountIndex).to.equal(0);
    expect(clientInstance.apps).to.deep.equal({dpns: {contractId: "7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM"}});
    expect(clientInstance.wallet.network).to.equal('testnet');
    expect(clientInstance.wallet.offlineMode).to.equal(false);
    expect(clientInstance.account.index).to.equal(0);
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
      expect(clientInstance.apps['dpns'].contractId).to.equal('7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM');
      expect(clientInstance.apps['dpns'].contractId).to.equal('7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM');
      return done();
    })
  });

  it('populate balance with dash', async () => {
    const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
    const faucetAddress = faucetPrivateKey
      .toAddress(process.env.NETWORK)
      .toString();

    await fundAddress(
      clientInstance.clients.dapi,
      faucetAddress,
      faucetPrivateKey,
      clientInstance.account.getAddress().address,
      20000
    )
  })

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

    createdIdentity = await clientInstance.platform.identities.register();

    createdIdentityId = createdIdentity.getId();

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
    expect(fetchIdentity.getId()).to.equal(createdIdentityId);

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
    expect(createDocument.getType()).to.equal('domain');
    expect(createDocument.getOwnerId()).to.equal(createdIdentityId);
    expect(createDocument.getDataContractId()).to.equal('7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM');
    expect(createDocument.get('label')).to.equal(username);
    expect(createDocument.get('normalizedParentDomainName')).to.equal('dash');
  });

  it('should retrieve itself by document', async function () {
    if(!createdIdentity){
      throw new Error('Can\'t perform the test. Failed to fetch identity & did not reg name');
    }

    const [doc] = await clientInstance.platform.documents.get('dpns.domain', {where:[
        ["normalizedParentDomainName","==","dash"],
        ["normalizedLabel","==",username.toLowerCase()],
      ]});

    expect(doc).to.exist;
    expect(doc.getRevision()).to.equal(1);
    expect(doc.getType()).to.equal('domain');
    expect(doc.getOwnerId()).to.equal(createdIdentityId);
    expect(doc.getDataContractId()).to.equal('7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM');
    expect(doc.get('label')).to.equal(username);
    expect(doc.get('normalizedParentDomainName')).to.equal('dash');
  });
  it('should disconnect', async function () {
    await clientInstance.disconnect();
  });
});

