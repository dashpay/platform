const { Api } = require('../../../src/index');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const rpcClient = require('../../../src/utils/RPCClient');
const { Bitcore } = require('../../../src');

const { TransitionHeader } = Bitcore.StateTransition;
const { Address } = Bitcore;
const { Registration: RegSubTx } = Bitcore.Transaction.SubscriptionTransactions;

chai.use(chaiAsPromised);
const { expect } = chai;

const validAddressWithOutputs = 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS';
const validAddressBalance = 1.01;
const validAddressWithoutOutputs = 'yVWnW3MY3QHNXgptKg1iQuCkqmtFhMGyPF';
const invalidAddress = '123';

const validUsername = 'Alice';
const notExistingUsername = 'Bob';
const invalidUsername = '1.2';

const validBlockHeight = 2357;
const validBlockHash = '6ce21c33e86c23dac892dab7be45ed791157d9463fbbb1bb45c9fe55a29d8bf8';

const validStateTransitionHex = '00000100018096980000000000fece053ccfee6b0e96083af22882ab3a5d420eb033c6adce5f9d70cca7258d3e0000000000000000000000000000000000000000000000000000000000000000fece053ccfee6b0e96083af22882ab3a5d420eb033c6adce5f9d70cca7258d3e0000';
const stateTransitionHash = 'f3bbe9211ac90a7079b9894b8abb49838c082c1bb5565fb87fb6001087794665';
const invalidStateTransitionHex = 'invalidtransitionhex';
const dataPacket = {};

const validTransactionHex = 'ffffffff0000ffffffff';
const transactionHash = 'a8502e9c08b3c851201a71d25bf29fd38a664baedb777318b12d19242f0e46ab';
const invalidTransactionHex = 'invalidtransactionhex';

function validateUsername(uname) {
  return uname.length >= 3 && uname.length <= 12 && /^[\x00-\x7F]+$/.test('uname');
}

describe('api', () => {

  before(() => {
    // stub requests to DAPI
    sinon.stub(rpcClient, 'request').callsFake(async function(url, method, params) {
      if (method === 'getUTXO') {
        if (!Address.isValid(params[0])) {
          throw new Error('Address is not valid');
        }
        if (params[0] === validAddressWithOutputs) {
          return [{}];
        }
        if (params[0] === validAddressWithoutOutputs) {
          return [];
        }
        throw new Error('Address not found');
      }
      if (method === 'getBalance') {
        if (!Address.isValid(params[0])) {
          throw new Error('Address is not valid');
        }
        if (params[0] === validAddressWithOutputs) {
          return validAddressBalance;
        }
        if (params[0] === validAddressWithoutOutputs) {
          return 0;
        }
        throw new Error('Address not found');
      }
      if (method === 'getUser') {
        /*
        Since dash schema uses fs, it would be impossible to run tests in browser
        with current version of validation from dash-schema
        */
        const isValidUsername = validateUsername(params[0]);
        const validRegTxId = false;
        if (isValidUsername) {
          if (params[0] === validUsername) {
            return {}; //todo
          }
          throw new Error('User with such username not found');
        }
        if (validRegTxId) {

        }
        throw new Error('Not found');
      }
      if (method === 'sendRawTransaction') {
        const transaction = new RegSubTx();
        transaction.fromString(params[0]);
        return transaction.toObject().hash;
      }
      if (method === 'sendRawTransition') {
        if (!params[1] || typeof params[1] !== 'object') {
          throw new Error('Data packet is missing');
        }
        const transitionHeader = new TransitionHeader().fromString(params[0]);
        return transitionHeader.toObject().tsid;
      }
      if (method === 'getBestBlockHeight') {
        return 100;
      }
      if (method === 'getBlockHash') {
        if (params[0] === validBlockHeight) {
          return validBlockHash;
        }
        throw new Error('Invalid block height');
      }
      if (method === 'getMNList') {
        return [];
      }
    });
  });

  after(() => {
    // Restore stubbed DAPI request
    rpcClient.request.restore();
  });

  describe('.address.getUTXO', () => {
    it('Should return list with unspent outputs for correct address, if there are any', async () => {
      const utxo = await Api.address.getUTXO(validAddressWithOutputs);
      expect(utxo).to.be.an('array');
      const output = utxo[0];
      expect(output).to.be.an('object');
    });
    it('Should return empty list if there is no unspent output', async () => {
      const utxo = await Api.address.getUTXO(validAddressWithoutOutputs);
      expect(utxo).to.be.an('array');
      expect(utxo.length).to.be.equal(0);
    });
    it('Should throw error if address is invalid', async () => {
      return expect(Api.address.getUTXO(invalidAddress)).to.be.rejected;
    });
    it('Should throw error if address not existing', async () => {
      return expect(Api.address.getUTXO(invalidAddress)).to.be.rejected;
    });
  });
  describe('.address.getBalance', () => {
    it('Should return sum of unspent outputs for address', async () => {
      const balance = await Api.address.getBalance(validAddressWithOutputs);
      expect(balance).to.be.equal(validAddressBalance);
    });
    it('Should return 0 if there is no unspent outputs', async () => {
      const balance = await Api.address.getBalance(validAddressWithoutOutputs);
      expect(balance).to.be.equal(0);
    });
    it('Should throw error if address is invalid', async () => {
      return expect(Api.address.getBalance(invalidAddress)).to.be.rejected;
    });
  });
  describe('.user.getUser', () => {
    it('Should throw error if username or regtx is incorrect', async () => {
      return expect(Api.user.getUser(invalidUsername)).to.be.rejected;
    });
    it('Should throw error if user not found', async () => {
      return expect(Api.user.getUser(notExistingUsername)).to.be.rejected;
    });
    it('Should return user data if user exists', async () => {
      const user = await Api.user.getUser(validUsername);
      expect(user).to.be.an('object');
    });
  });
  describe('.transaction.sendRaw', () => {
    it('Should return hash of transaction', async () => {
      const hash = await Api.transaction.sendRaw(validTransactionHex);
      expect(hash).to.be.equal(transactionHash);
    });
    it('Should throw error if hex is wrong', async () => {
      return expect(Api.transaction.sendRaw(invalidTransactionHex)).to.be.rejected;
    });
  });
  describe('.stateTransition.sendRaw', () => {
    it('Should return hash of state transition', async () => {
      const hash = await Api.stateTransition.sendRaw(validStateTransitionHex, dataPacket);
      expect(hash).to.be.equal(stateTransitionHash);
    });
    it('Should throw error if data packet is missing', async () => {
      return expect(Api.stateTransition.sendRaw(validStateTransitionHex)).to.be.rejected;
    });
    it('Should throw error if hex is wrong', async () => {
      return expect(Api.stateTransition.sendRaw(invalidStateTransitionHex)).to.be.rejected;
    });
  });
  describe('.block.getBestBlockHeight', () => {
    it('Should return block height', async () => {
      const bestBlockHeight = await Api.block.getBestBlockHeight();
      expect(bestBlockHeight).to.be.a('number');
    });
  });
  describe('.block.getBlockHash', () => {
    it('Should return hash for a given block height', async () => {
      const blockHash = await Api.block.getBlockHash(2357);
      expect(blockHash).to.be.a('string');
    });
    it('Should be rejected if height is invalid', async () => {
      await expect(Api.block.getBlockHash(1000000)).to.be.rejected;
      await expect(Api.block.getBlockHash('some string')).to.be.rejected;
    });
  });
});