import { expect } from 'chai';
import {Client} from "./index";
import 'mocha';

const mnemonic = 'agree country attract master mimic ball load beauty join gentle turtle hover';
describe('Dash - Client', function suite() {
  this.timeout(10000);
  it('should provide expected class', function () {
    expect(Client.name).to.be.equal('Client');
    expect(Client.constructor.name).to.be.equal('Function');
  });
  it('should be instantiable', function () {
    const client = new Client();
    expect(client).to.exist;
    expect(client.network).to.be.equal('testnet');
    expect(client.getDAPIClient().constructor.name).to.be.equal('DAPIClient');
  });
  it('should not initiate wallet lib without mnemonic', function () {
    const client = new Client();
    expect(client.wallet).to.be.equal(undefined);
  });
  it('should initiate wallet-lib with a mnemonic', async ()=>{
    const client = new Client({
      wallet: {
        mnemonic,
        offlineMode: true,
      }
    });
    expect(client.wallet).to.exist;
    expect(client.wallet!.offlineMode).to.be.equal(true);

    // @ts-ignore
    await client.wallet.storage.stopWorker();
    // @ts-ignore
    await client.wallet.disconnect();

    const account = await client.getWalletAccount();
    await account.disconnect();
  });
});
