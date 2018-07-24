const Api = require('../../src/index');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const rpcClient = require('../../src/RPCClient');

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

const validBlockchainUserObject = {
    uname: validUsername,
    regtxid: 'e1abfdbda9e0204573f03c8c354c40649c711253ec3c978011ef320bd5bbc33a',
    pubkeyid: 'd7d295e04202cc652d845cc51762dc64a5fd2bdc',
    credits: 10000,
    data: '0000000000000000000000000000000000000000000000000000000000000000',
    state: 'open',
    subtx:
        ['e1abfdbda9e0204573f03c8c354c40649c711253ec3c978011ef320bd5bbc33a'],
    transitions: [],
    from_mempool: true
};

function validateUsername(uname) {
  return uname.length >= 3 && uname.length <= 12 && /^[\x00-\x7F]+$/.test('uname');
}

describe('api', () => {

  before(() => {
    // stub requests to DAPI
    sinon.stub(rpcClient, 'request').callsFake(async function(url, method, params) {
      const {
        address, username, userId, rawTransaction, rawTransitionHeader, rawTransitionPacket, height
      } = params;
      if (method === 'getUTXO') {
        if (address === validAddressWithOutputs) {
          return [{}];
        }
        if (address === validAddressWithoutOutputs) {
          return [];
        }
        throw new Error('Address not found');
      }
      if (method === 'getBalance') {
        if (address === validAddressWithOutputs) {
          return validAddressBalance;
        }
        if (address === validAddressWithoutOutputs) {
          return 0;
        }
        throw new Error('Address not found');
      }
        if (method === 'getUser') {
            /*
            Since dash schema uses fs, it would be impossible to run tests in browser
            with current version of validation from dash-schema
            */
            if (username !== undefined) {
                const isValidUsername = validateUsername(username);
                if (isValidUsername) {
                    if (username === validUsername) {
                        return validBlockchainUserObject;
                    }
                }
                throw new Error('User with such username not found');
            } else {
                if (userId === validBlockchainUserObject.regtxid) {
                    return validBlockchainUserObject;
                }
                throw new Error('User with such od not found');
            }
            throw new Error('Not found');
        }
      if (method === 'sendRawTransition') {
        if (!rawTransitionHeader || typeof rawTransitionPacket !== 'object') {
          throw new Error('Data packet is missing');
        }
        const transitionHeader = new TransitionHeader().fromString(rawTransitionHeader);
        return transitionHeader.toObject().tsid;
      }
      if (method === 'getBestBlockHeight') {
        return 100;
      }
      if (method === 'getBlockHash') {
        if (height === validBlockHeight) {
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

  describe('constructor', () => {
    it('Should set seeds and port, if passed', async () => {
      const dapi = new Api({seeds: [{ip:'127.1.2.3'}], port: 1234});
      expect(dapi.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.masternodeList).to.be.deep.equal([{ip:'127.1.2.3'}]);
      expect(dapi.MNDiscovery.seeds).to.be.deep.equal([{ip:'127.1.2.3'}]);

      await dapi.getBestBlockHeight();
      expect(rpcClient.request.calledWith({ host: '127.1.2.3', port: 1234 }, 'getMNList', {})).to.be.true;
      expect(rpcClient.request.calledWith({ host: '127.1.2.3', port: 1234 }, 'getBestBlockHeight', {})).to.be.true;
    });
  });

  describe('.address.getUTXO', () => {
    it('Should return list with unspent outputs for correct address, if there are any', async () => {
      const dapi = new Api();
      const utxo = await dapi.getUTXO(validAddressWithOutputs);
      expect(utxo).to.be.an('array');
      const output = utxo[0];
      expect(output).to.be.an('object');
    });
    it('Should return empty list if there is no unspent output', async () => {
      const dapi = new Api();
      const utxo = await dapi.getUTXO(validAddressWithoutOutputs);
      expect(utxo).to.be.an('array');
      expect(utxo.length).to.be.equal(0);
    });
    it('Should throw error if address is invalid', async () => {
      const dapi = new Api();
      return expect(dapi.getUTXO(invalidAddress)).to.be.rejected;
    });
    it('Should throw error if address not existing', async () => {
      const dapi = new Api();
      return expect(dapi.getUTXO(invalidAddress)).to.be.rejected;
    });
  });
  describe('.address.getBalance', () => {
    it('Should return sum of unspent outputs for address', async () => {
      const dapi = new Api();
      const balance = await dapi.getBalance(validAddressWithOutputs);
      expect(balance).to.be.equal(validAddressBalance);
    });
    it('Should return 0 if there is no unspent outputs', async () => {
      const dapi = new Api();
      const balance = await dapi.getBalance(validAddressWithoutOutputs);
      expect(balance).to.be.equal(0);
    });
    it('Should throw error if address is invalid', async () => {
      const dapi = new Api();
      return expect(dapi.getBalance(invalidAddress)).to.be.rejected;
    });
  });
  describe('.user.getUserByName', () => {
    it('Should throw error if username or regtx is incorrect', async () => {
      const dapi = new Api();
      return expect(dapi.getUserByName(invalidUsername)).to.be.rejected;
    });
    it('Should throw error if user not found', async () => {
      const dapi = new Api();
      return expect(dapi.getUserByName(notExistingUsername)).to.be.rejected;
    });
    it('Should return user data if user exists', async () => {
      const dapi = new Api();
      const user = await dapi.getUserByName(validUsername);
      expect(user).to.be.an('object');
    });
  });
  describe('.user.getUserById', () => {
      it('Should throw error if use id is incorrect', async () => {
          const dapi = new Api();
          const user= await dapi.getUserByName(validUsername);
          dapi.generate(10);
          return expect(dapi.getUserById(user.regtxid+"fake")).to.be.rejected;
      });
      it('Should throw error if user id not found', async () => {
          const dapi = new Api();
          return expect(dapi.getUserById(notExistingUsername)).to.be.rejected;
        });
        it('Should return user data if user exists', async () => {
            const dapi = new Api();
            const user = await dapi.getUserByName(validUsername);
            const userById = await dapi.getUserById(user.regtxid)
            expect(userById).to.be.an('object');
        });
    });
  describe('.block.getBestBlockHeight', () => {
    it('Should return block height', async () => {
      const dapi = new Api();
      const bestBlockHeight = await dapi.getBestBlockHeight();
      expect(bestBlockHeight).to.be.a('number');
        expect(bestBlockHeight).to.be.equal(100);
    });
  });
  describe('.block.getBlockHash', () => {
    it('Should return hash for a given block height', async () => {
      const dapi = new Api();
      const blockHash = await dapi.getBlockHash(2357);
      expect(blockHash).to.be.a('string');
      expect(blockHash).to.be.equal(validBlockHash);
    });
    it('Should be rejected if height is invalid', async () => {
      const dapi = new Api();
      await expect(dapi.getBlockHash(1000000)).to.be.rejected;
      await expect(dapi.getBlockHash('some string')).to.be.rejected;
      await expect(dapi.getBlockHash(1.2)).to.be.rejected;
      await expect(dapi.getBlockHash(-1)).to.be.rejected;
      await expect(dapi.getBlockHash(true)).to.be.rejected;
    });
  });
});