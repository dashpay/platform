import { expect } from 'chai';
import { Client } from "./index";
import 'mocha';
import { Transaction } from "@dashevo/dashcore-lib";
import { createFakeInstantLock } from "../../utils/createFakeIntantLock";
import Identity from '@dashevo/dpp/lib/identity/Identity';
import { createDapiClientMock } from "../../test/mocks/createDapiClientMock";

// @ts-ignore
const TxStreamMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamMock');
// @ts-ignore
const TransportMock = require('@dashevo/wallet-lib/src/test/mocks/TransportMock');
// @ts-ignore
const TxStreamDataResponseMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamDataResponseMock');

const mnemonic = 'agree country attract master mimic ball load beauty join gentle turtle hover';
describe('Dash - Client', function suite() {
  this.timeout(30000);

  let txStreamMock;
  let transportMock;
  let testHDKey;
  let clientWithMockTransport;
  let account;
  let walletTransaction;
  let dapiClientMock;

  beforeEach(async function beforeEach() {
    txStreamMock = new TxStreamMock();
    transportMock = new TransportMock(this.sinon, txStreamMock);
    testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";

    transportMock.getIdentityIdsByPublicKeyHash.returns([null]);

    dapiClientMock = createDapiClientMock(this.sinon);

    clientWithMockTransport = new Client({
      wallet: {
        HDPrivateKey: testHDKey,
      }
    });
    // Mock wallet transport for transactions
    clientWithMockTransport.wallet.transport = transportMock;
    // Mock dapi client for platform endpoints
    clientWithMockTransport.dapiClient = dapiClientMock;

    // setInterval(() => {
    //   txStreamMock.emit(TxStreamMock.EVENTS.end);
    // }, 100);

    [account] = await Promise.all([
      clientWithMockTransport.wallet.getAccount(),
      new Promise(resolve => {
        setTimeout(() => {
          txStreamMock.emit(TxStreamMock.EVENTS.end);
          resolve();
        }, 100)
      })
    ]);
    // account = await clientWithMockTransport.wallet.getAccount();

    // add fake tx to the wallet so it will be able to create transactions
    walletTransaction = new Transaction(undefined)
        .from([{
          amount: 150000,
          script: '76a914f9996443a7d5e2694560f8715e5e8fe602133c6088ac',
          outputIndex: 0,
          txid: new Transaction(undefined).hash,
        }])
        .to(account.getAddress(10).address, 100000);

    await account.importTransactions([walletTransaction.serialize(true)]);
  });

  it('should provide expected class', function () {
    expect(Client.name).to.be.equal('Client');
    expect(Client.constructor.name).to.be.equal('Function');
  });
  it('should be instantiable', function () {
    const client = new Client();
    expect(client).to.exist;
    expect(client.network).to.be.equal('evonet');
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

    await client.wallet?.storage.stopWorker();
    await client.wallet?.disconnect();

    const account = await client.getWalletAccount();
    await account.disconnect();
  });
  it('should throw an error if client and wallet have different networks', async () => {
    try {
      new Client({
        network: 'evonet',
        wallet: {
          mnemonic,
          offlineMode: true,
          network: 'testnet',
        },
      });

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.equal('Wallet and Client networks are different');
    }
  });

  describe('#platform.identities.register', async () => {
    it('should register an identity', async () => {
      // Set up transport to emit instant lock when it receives transaction
      let isLock;
      let transaction;
      transportMock.sendTransaction.callsFake((txString) => {
        transaction = new Transaction(txString);

        isLock = createFakeInstantLock(transaction.hash);
        txStreamMock.emit(
            TxStreamMock.EVENTS.data,
            new TxStreamDataResponseMock(
                { instantSendLockMessages: [isLock.toBuffer()] }
            )
        );
      });

      // Set up DAPI mock to return identity
      let interceptedIdentityStateTransition;
      let identityFromDAPI;
      dapiClientMock.platform.broadcastStateTransition.callsFake(async (stBuffer) => {
        interceptedIdentityStateTransition = await clientWithMockTransport.platform.dpp.stateTransition.createFromBuffer(stBuffer);
        identityFromDAPI = new Identity({
          protocolVersion: interceptedIdentityStateTransition.getProtocolVersion(),
          id: interceptedIdentityStateTransition.getIdentityId().toBuffer(),
          publicKeys: interceptedIdentityStateTransition.getPublicKeys().map((key) => key.toObject()),
          balance: 100,
          revision: 0,
        });
        dapiClientMock.platform.getIdentity.resolves(identityFromDAPI);
      });

      const [identity] = await Promise.all([
        clientWithMockTransport.platform.identities.register(),
      ]);

      expect(identity).to.be.not.null;

      const interceptedAssetLock = interceptedIdentityStateTransition.getAssetLock();

      // Check intercepted st
      expect(interceptedAssetLock.getProof().getInstantLock()).to.be.deep.equal(isLock);
      expect(interceptedAssetLock.getTransaction().hash).to.be.equal(transaction.hash);

      const importedIdentityIds = account.getIdentityIds();
      expect(importedIdentityIds.length).to.be.equal(1);
      expect(importedIdentityIds[0]).to.be.equal(interceptedIdentityStateTransition.getIdentityId().toString());
    });
  });
});
