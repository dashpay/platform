import { expect } from 'chai';
import getResponseMetadataFixture from '../../test/fixtures/getResponseMetadataFixture';
import { Client } from "./index";
import 'mocha';
import { Transaction, BlockHeader } from "@dashevo/dashcore-lib";
import { createFakeInstantLock } from "../../utils/createFakeIntantLock";
import stateTransitionTypes from '@dashevo/dpp/lib/stateTransition/stateTransitionTypes';
import { StateTransitionBroadcastError } from '../../errors/StateTransitionBroadcastError';

// @ts-ignore
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
// @ts-ignore
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const GetIdentityResponse = require("@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse");
const GetDataContractResponse = require("@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse");

import { createIdentityFixtureInAccount } from '../../test/fixtures/createIdentityFixtureInAccount';
import { createTransactionInAccount } from '../../test/fixtures/createTransactionFixtureInAccount';
import { createAndAttachTransportMocksToClient } from '../../test/mocks/createAndAttachTransportMocksToClient';

const blockHeaderFixture = '00000020e2bddfb998d7be4cc4c6b126f04d6e4bd201687523ded527987431707e0200005520320b4e263bec33e08944656f7ce17efbc2c60caab7c8ed8a73d413d02d3a169d555ecdd6021e56d000000203000500010000000000000000000000000000000000000000000000000000000000000000ffffffff050219250102ffffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac0000000046020019250000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f8000000000000000000000000000000000000000000000000000000000000000003000600000000000000fd4901010019250000010001d02e9ee1b14c022ad6895450f3375a8e9a87f214912d4332fa997996d2000000320000000000000032000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';

describe('Dash - Client', function suite() {
  this.timeout(30000);

  let testMnemonic;
  let txStreamMock;
  let transportMock;
  let testHDKey;
  let client;
  let account;
  let walletTransaction;
  let dapiClientMock;
  let identityFixture;
  let documentsFixture;
  let dataContractFixture;

  beforeEach(async function beforeEach() {
    testMnemonic = 'agree country attract master mimic ball load beauty join gentle turtle hover';
    testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";

    client = new Client({
      wallet: {
        HDPrivateKey: testHDKey,
      }
    });

    ({ txStreamMock, transportMock, dapiClientMock } = await createAndAttachTransportMocksToClient(client, this.sinon));

    account = await client.getWalletAccount();

    // add fake tx to the wallet so it will be able to create transactions
    walletTransaction = await createTransactionInAccount(account);
    // create an identity in the account so we can sign state transitions
    identityFixture = createIdentityFixtureInAccount(account);
    dataContractFixture = getDataContractFixture();
    documentsFixture = getDocumentsFixture(dataContractFixture);

    transportMock.getTransaction.resolves({
      transaction: new Transaction('03000000019ecd68f367aba679209b9c912ff1d2ef9147f90eba2a47b5fb0158e27fb15476000000006b483045022100af2ca966eaeef8f5493fd8bcf2248d60b3f6b8236c137e2d099c8ba35878bf9402204f653232768eb8b06969b13f0aa3579d653163f757009e0c261c9ffd32332ffb0121034244016aa525c632408bc627923590cf136b47035cd57aa6f1fa8b696d717304ffffffff021027000000000000166a140f177a991f37fe6cbb08fb3f21b9629fa47330e3a85b0100000000001976a914535c005bfef672162aa2c53f0f6630a57ade344588ac00000000'),
      blockHash: Buffer.from('0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', 'hex'),
      height: 42,
      confirmations: 10,
      isInstantLocked: true,
      isChainLocked: false,
    });

    transportMock.getBlockHeaderByHash
        .returns(BlockHeader.fromString(blockHeaderFixture));

    dapiClientMock.platform.getIdentity.resolves(new GetIdentityResponse(identityFixture.toBuffer(), getResponseMetadataFixture()));
    dapiClientMock.platform.getDataContract.resolves(new GetDataContractResponse(dataContractFixture.toBuffer(), getResponseMetadataFixture()));
  });

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
        mnemonic: testMnemonic,
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
        network: 'testnet',
        wallet: {
          mnemonic: testMnemonic,
          offlineMode: true,
          network: 'evonet',
        },
      });

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.equal('Wallet and Client networks are different');
    }
  });

  describe('#platform.identities.register', async () => {
    it('should register an identity', async () => {
      const accountIdentitiesCountBeforeTest = account.identities.getIdentityIds().length;

      const identity = await client.platform.identities.register();

      expect(identity).to.be.not.null;

      const serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
      const interceptedIdentityStateTransition = await client.platform.dpp.stateTransition.createFromBuffer(serializedSt);
      const interceptedAssetLockProof = interceptedIdentityStateTransition.getAssetLockProof();

      const transaction = new Transaction(transportMock.sendTransaction.getCall(0).args[0]);
      const isLock = createFakeInstantLock(transaction.hash);

      // Check intercepted st
      expect(interceptedAssetLockProof.getInstantLock()).to.be.deep.equal(isLock);
      expect(interceptedAssetLockProof.getTransaction().hash).to.be.equal(transaction.hash);

      const importedIdentityIds = account.identities.getIdentityIds();
      // Check that we've imported identities properly
      expect(importedIdentityIds.length).to.be.equal(accountIdentitiesCountBeforeTest + 1);
      expect(importedIdentityIds[0]).to.be.equal(interceptedIdentityStateTransition.getIdentityId().toString());
    });

    it('should throw TransitionBroadcastError when transport resolves error', async () => {
      const accountIdentitiesCountBeforeTest = account.identities.getIdentityIds().length;

      const errorResponse = {
        error: {
          code: 2,
          message: "Error happened",
          data: {},
        }
      };

      dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);

      let error;
      try {
        await client.platform.identities.register();
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(error.getCode()).to.be.equal(errorResponse.error.code);
      expect(error.getMessage()).to.be.equal(errorResponse.error.message);

      const importedIdentityIds = account.identities.getIdentityIds();
      // Check that no identities were imported
      expect(importedIdentityIds.length).to.be.equal(accountIdentitiesCountBeforeTest);
    });
  });

  describe('#platform.identities.topUp', async () => {
    it('should top up an identity', async () => {
      // Registering an identity we're going to top up
      const identity = await client.platform.identities.register();
      // Topping up the identity
      await client.platform.identities.topUp(identity.getId(), 10000);

      expect(identity).to.be.not.null;

      const serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(1).args[0];
      const interceptedIdentityStateTransition = await client.platform.dpp.stateTransition.createFromBuffer(serializedSt);
      const interceptedAssetLockProof = interceptedIdentityStateTransition.getAssetLockProof();

      expect(interceptedIdentityStateTransition.getType()).to.be.equal(stateTransitionTypes.IDENTITY_TOP_UP);

      const transaction = new Transaction(transportMock.sendTransaction.getCall(1).args[0]);
      const isLock = createFakeInstantLock(transaction.hash);
      // Check intercepted st
      expect(interceptedAssetLockProof.getInstantLock()).to.be.deep.equal(isLock);
      expect(interceptedAssetLockProof.getTransaction().hash).to.be.equal(transaction.hash);
    });

    it('should throw TransitionBroadcastError when transport resolves error', async () => {
      // Registering an identity we're going to top up
      const identity = await client.platform.identities.register();

      const errorResponse = {
        error: {
          code: 2,
          message: "Error happened",
          data: {},
        }
      };

      dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);

      let error;
      try {
        // Topping up the identity
        await client.platform.identities.topUp(identity.getId(), 10000);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(error.getCode()).to.be.equal(errorResponse.error.code);
      expect(error.getMessage()).to.be.equal(errorResponse.error.message);
    });
  });

  describe('#platform.documents.broadcast', () => {
    it('should throw TransitionBroadcastError when transport resolves error', async () => {
      const errorResponse = {
        error: {
          code: 2,
          message: "Error happened",
          data: {},
        }
      };

      dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);

      let error;
      try {
        await client.platform.documents.broadcast({
          create: documentsFixture,
        }, identityFixture);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(error.getCode()).to.be.equal(errorResponse.error.code);
      expect(error.getMessage()).to.be.equal(errorResponse.error.message);
    });

    it('should broadcast documents', async () => {
      const proofResponse = {
        proof: { }
      }

      dapiClientMock.platform.waitForStateTransitionResult.resolves(proofResponse);

      await client.platform.documents.broadcast({
        create: documentsFixture,
      }, identityFixture);

      const serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
      const interceptedSt = await client.platform.dpp.stateTransition.createFromBuffer(serializedSt);

      // .to.be.true() doesn't work after TS compilation in Chrome
      expect(interceptedSt.verifySignature(identityFixture.getPublicKeyById(0))).to.be.equal(true);

      const documentTransitions = interceptedSt.getTransitions();

      expect(documentTransitions.length).to.be.greaterThan(0);
      expect(documentTransitions.length).to.be.equal(documentsFixture.length);
    });
  });

  describe('#platform.contracts.broadcast', () => {
    it('should throw TransitionBroadcastError when transport resolves error', async () => {
      const errorResponse = {
        error: {
          code: 2,
          message: "Error happened",
          data: {},
        }
      };

      dapiClientMock.platform.waitForStateTransitionResult.resolves(errorResponse);

      let error;
      try {
        await client.platform.contracts.broadcast(dataContractFixture, identityFixture);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(error.getCode()).to.be.equal(errorResponse.error.code);
      expect(error.getMessage()).to.be.equal(errorResponse.error.message);
    });

    it('should broadcast data contract', async () => {
      dapiClientMock.platform.waitForStateTransitionResult.resolves({
        proof: {  }
      });

      await client.platform.contracts.broadcast(dataContractFixture, identityFixture);

      const serializedSt = dapiClientMock.platform.broadcastStateTransition.getCall(0).args[0];
      const interceptedSt = await client.platform.dpp.stateTransition.createFromBuffer(serializedSt);

      // .to.be.true() doesn't work after TS compilation in Chrome
      expect(interceptedSt.verifySignature(identityFixture.getPublicKeyById(0))).to.be.equal(true);
      expect(interceptedSt.getEntropy()).to.be.deep.equal(dataContractFixture.entropy);
      expect(interceptedSt.getDataContract().toObject()).to.be.deep.equal(dataContractFixture.toObject());
    });
  });
});
