const {expect} = require('chai');
const Dash = require(typeof process === 'undefined' ? '../../src/index.ts' : '../../');
const Chance = require('chance');
const chance = new Chance();
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

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

  // We take random coz two browsers run in parallel
  // and they can take the same inputs

  const inputIndex = Math.floor(
    Math.random() * Math.floor(inputs.length / 2) * -1,
  );

  const transaction = new Transaction();

  transaction.from(inputs.slice(inputIndex)[0])
    .to(address, amount)
    .change(faucetAddress)
    .fee(668)
    .sign(faucetPrivateKey);

  let { blocks: currentBlockHeight } = await dapiClient.getStatus();

  const transactionId = await dapiClient.sendTransaction(transaction.toBuffer());

  const desiredBlockHeight = currentBlockHeight + 1;

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

const seeds = process.env.DAPI_SEED
  .split(',')
  .map((seed) => ({ service: seed }));

const clientOpts = {
  seeds,
  network: process.env.NETWORK,
  wallet: {
    mnemonic: null,
  },
  apps: {
    dpns: {
      contractId: process.env.DPNS_CONTRACT_ID,
    }
  }
};

let account;

describe('SDK', function suite() {
  this.timeout(700000);

  it('should init a Client', async () => {
    clientInstance = new Dash.Client(clientOpts);
    expect(clientInstance.network).to.equal(process.env.NETWORK);
    expect(clientInstance.walletAccountIndex).to.equal(0);
    expect(clientInstance.apps).to.deep.equal({dpns: {contractId: process.env.DPNS_CONTRACT_ID}});
    expect(clientInstance.wallet.network).to.equal(process.env.NETWORK);
    expect(clientInstance.wallet.offlineMode).to.equal(false);
    expect(clientInstance.platform.dpp).to.exist;
    expect(clientInstance.platform.client).to.exist;

    account = await clientInstance.getWalletAccount();
    expect(account.index).to.equal(0);
  });

  it('should sign and verify a message', async function () {
    const idKey = account.getIdentityHDKey();
    // This transforms from a Wallet-Lib.PrivateKey to a Dashcore-lib.PrivateKey.
    // It will quickly be annoying to perform this, and we therefore need to find a better solution for that.
    const privateKey = Dash.Core.PrivateKey(idKey.privateKey);
    const message = Dash.Core.Message('hello, world');
    const signed = message.sign(privateKey);
    const verify = message.verify(idKey.privateKey.toAddress().toString(), signed.toString());
    expect(verify).to.equal(true);
  });

  it('populate balance with dash', async () => {
    const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
    const faucetAddress = faucetPrivateKey
      .toAddress(process.env.NETWORK)
      .toString();

    await fundAddress(
      clientInstance.getDAPIClient(),
      faucetAddress,
      faucetPrivateKey,
      account.getAddress().address,
      50000
    )
  })

  it('should have a balance', function (done) {
    const balance = (account.getTotalBalance());
    if(balance<10000){
      return done(new Error(`You need to fund this address : ${account.getUnusedAddress().address}. Insuffisiant balance: ${balance}`));
    }
    hasBalance = true;
    return done();
  });

  it('should check if name is available' , async function () {
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
    expect(createDocument.getDataContractId()).to.equal(process.env.DPNS_CONTRACT_ID);
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
    expect(doc.getDataContractId()).to.equal(process.env.DPNS_CONTRACT_ID);
    expect(doc.get('label')).to.equal(username);
    expect(doc.get('normalizedParentDomainName')).to.equal('dash');
  });
  it('should create and broadcast contract', async () => {
    if(!createdIdentity){
      throw new Error('Can\'t perform the test. Failed to fetch identity & did not reg name');
    }

    const documentsDefinition = {
      test: {
        properties: {
          testProperty: {
            type: "string"
          }
        },
        additionalProperties: false,
      }
    }

    const contract = await clientInstance.platform.contracts.create(documentsDefinition, createdIdentity);

    expect(contract).to.exist;
    expect(contract).to.be.instanceOf(DataContract);

    await clientInstance.platform.contracts.broadcast(contract, createdIdentity);

    await wait(1000);

    const fetchedContract = await clientInstance.platform.contracts.get(contract.getId());

    expect(fetchedContract).to.exist;
    expect(fetchedContract).to.be.instanceOf(DataContract);
    expect(fetchedContract.toJSON()).to.be.deep.equal(contract.toJSON());
  });

  it('should top up identity', async function () {
    const identityId = createdIdentity.getId();

    const identityBeforeTopUp = await clientInstance.platform.identities.get(identityId);
    const balanceBeforeTopUp = identityBeforeTopUp.getBalance();
    const topUpAmount = 10000;
    const topUpCredits = topUpAmount * 1000;

    await clientInstance.platform.identities.topUp(identityId, topUpAmount);

    const identity = await clientInstance.platform.identities.get(identityId);

    expect(identity.getId()).to.be.equal(identityId);

    // Fee is based on ST's size atm, so we too lazy
    // to take it somehow from clientInstance.platform.identities.topUp

    expect(identity.getBalance()).to.be.greaterThan(balanceBeforeTopUp);
    expect(identity.getBalance()).to.be.lessThan(balanceBeforeTopUp + topUpCredits);
  })

  it('should disconnect', async function () {
    await clientInstance.disconnect();
  });
});
