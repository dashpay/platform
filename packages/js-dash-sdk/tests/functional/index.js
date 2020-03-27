const {expect} = require('chai');
const Dash = require('../../dist/dash.cjs.min');

describe('SDK', function suite() {
  this.timeout(10000);
  let instanceWithoutWallet = {};
  let instanceWithWallet = {};
  it('should provide expected class', function () {
    expect(Dash).to.have.property('Client');
    expect(Dash.Client.constructor.name).to.be.equal('Function')
  });
  it('should create an instance', function (done) {
    instanceWithoutWallet = new Dash.Client();
    expect(instanceWithoutWallet.network).to.equal('testnet');
    expect(instanceWithoutWallet.apps).to.deep.equal({
          dpns: { contractId: '77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3' }
        }
    );

    instanceWithWallet = new Dash.Client({mnemonic:null});
    expect(instanceWithWallet.network).to.equal('testnet');
    expect(instanceWithWallet.apps).to.deep.equal({
          dpns: { contractId: '77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3' }
        }
    );
    expect(instanceWithWallet.wallet.mnemonic).to.exist;
    done();
  });
  it('should sign and verify a message', function () {
    const {account} = instanceWithWallet;
    const idKey = account.getIdentityHDKey();
    // This transforms from a Wallet-Lib.PrivateKey to a Dashcore-lib.PrivateKey.
    // It will quickly be annoying to perform this, and we therefore need to find a better solution for that.
    const privateKey = Dash.Core.PrivateKey(idKey.privateKey);
    const message = Dash.Core.Message('hello, world');
    const signed = message.sign(privateKey);
    const verify = message.verify(idKey.privateKey.toAddress().toString(), signed.toString());
    expect(verify).to.equal(true);
  });
  after(async ()=>{
    await instanceWithWallet.isReady();
    await instanceWithWallet.disconnect();
    await instanceWithoutWallet.disconnect();
  })
});
