const {expect} = require('chai');
const Dash = require(typeof process === 'undefined' ? '../../src/index.ts' : '../../');

const {
  Networks,
} = require('@dashevo/dashcore-lib');

let clientInstance;

const seeds = process.env.DAPI_SEED
  .split(',');

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
    expect(clientInstance.wallet.network).to.equal(Networks.get(process.env.NETWORK).name);
    expect(clientInstance.wallet.offlineMode).to.equal(false);
    expect(clientInstance.platform.dpp).to.exist;
    expect(clientInstance.platform.client).to.exist;

    account = await clientInstance.getWalletAccount();
    expect(account.index).to.equal(0);
  });

  it('should sign and verify a message', async function () {
    const idKey = account.getIdentityHDKeyByIndex(0, 0);
    // This transforms from a Wallet-Lib.PrivateKey to a Dashcore-lib.PrivateKey.
    // It will quickly be annoying to perform this, and we therefore need to find a better solution for that.
    const privateKey = Dash.Core.PrivateKey(idKey.privateKey);
    const message = Dash.Core.Message('hello, world');
    const signed = message.sign(privateKey);
    const verify = message.verify(idKey.privateKey.toAddress().toString(), signed.toString());
    expect(verify).to.equal(true);
  });

  it('should disconnect', async function () {
    await clientInstance.disconnect();
  });
});
