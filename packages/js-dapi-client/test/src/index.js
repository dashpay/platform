const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const Api = require('../../src/index');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const rpcClient = require('../../src/RPCClient');
const config = require('../../src/config');
const SMNListFixture = require('../fixtures/mnList');

const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const doubleSha256 = require('../utils/doubleSha256');

chai.use(chaiAsPromised);
const { expect } = chai;

const validAddressWithOutputs = 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS';
const validAddressSummary = {
  'addrStr': validAddressWithOutputs,
  'balance': 4173964.74940914,
  'balanceSat': 417396474940914,
  'totalReceived': 4287576.24940914,
  'totalReceivedSat': 428757624940914,
  'totalSent': 113611.5,
  'totalSentSat': 11361150000000,
  'unconfirmedBalance': 0,
  'unconfirmedBalanceSat': 0,
  'unconfirmedTxApperances': 0,
  'txApperances': 27434,
  'transactions': ['4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', '8890a0ee95a17f6723ab2d9a0bdd579351b9220738ad34f5b49cbe63f09b082a']
};
const validAddressTransactions = {
  'totalItems': 27434,
  'from': 0,
  'to': 10,
  'items': [
    {
      'txid': '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176',
      'version': 2,
      'locktime': 16624,
      'vin': [],
      'vout': [],
      'blockhash': '0000037e4114e62941af4d5c34055561917520ece2a261297df892909f635c61',
      'blockheight': -1,
      'confirmations': 0,
      'time': 1545636907,
      'valueOut': 206.55000101,
      'size': 520,
      'valueIn': 206.55000624,
      'fees': 0.00000523,
      'txlock': false
    },
    {
      'txid': '8890a0ee95a17f6723ab2d9a0bdd579351b9220738ad34f5b49cbe63f09b082a',
      'version': 3,
      'locktime': 0,
      'vin': [],
      'vout': [],
      'blockhash': '0000037e4114e62941af4d5c34055561917520ece2a261297df892909f635c61',
      'blockheight': -1,
      'confirmations': 0,
      'time': 1545636907,
      'valueOut': 499.9999,
      'size': 297,
      'valueIn': 500,
      'fees': 0.0001,
      'txlock': false
    }]
};

const historicBlockchainDataSyncStatus =
  {
    'status': 'syncing',
    'blockChainHeight': 16322,
    'syncPercentage': 86,
    'height': 16322,
    'error': null,
    'type': 'bitcore node'
  };

const rawBlock = {
  'rawblock': '0000037e4114e62941af4d5c34055561917520ece2a261297df892909f635c61'
};

const dapId = '11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c';

const dapContract = {
  'dapname': 'TestContacts_test',
  'dapver': 1,
  'idx': 0,
  'meta': {
    'id': dapId
  },
  'pver': 1,
  'dapschema': {
    'title': 'TestContacts_test',
    'DashPay': {
      '$id': 'http://dash.org/schemas/sys/dapschema',
      'user': {
        '$id': 'http://dash.org/schemas/sys/dapobject',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        '_isrole': true,
        'properties': {
          'avatar': {
            'type': 'string'
          },
          'aboutme': {
            'type': 'string'
          }
        }
      },
      'store': {
        '$id': 'http://dash.org/schemas/sys/dapobject',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        '_isrole': true,
        'properties': {
          'storename': {
            'type': 'number'
          }
        }
      },
      'title': 'DashPay',
      'contact': {
        '$id': 'http://dash.org/schemas/sys/dapobject',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        'required': [
          'hdextpubkey',
          'user'
        ],
        'properties': {
          'pub': {
            'type': 'string'
          },
          'user': {
            '$ref': 'http://dash.org/schemas/sys#/definitions/relation'
          },
          'hdextpubkey': {
            'type': 'string'
          }
        }
      }
    },
    'MemoDash': {
      '$id': 'http://dash.org/schema/dap',
      'like': {
        'type': 'object',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        'required': [
          'relation'
        ],
        'properties': {
          'relation': {
            '$ref': 'http://dash.org/schemas/sys#/definitions/relation'
          },
          'tipTxHash': {
            'type': 'string'
          }
        }
      },
      'memo': {
        'type': 'object',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        'required': [
          'message',
          'createdAt'
        ],
        'properties': {
          'message': {
            'type': 'string',
            'maxLength': 144,
            'minLength': 1
          },
          'relation': {
            '$ref': 'http://dash.org/schemas/sys#/definitions/relation'
          },
          'updateAt': {
            'type': 'string',
            'format': 'date-time'
          },
          'createdAt': {
            'type': 'string',
            'format': 'date-time'
          }
        }
      },
      'title': 'DashMemo',
      'follow': {
        'type': 'object',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        'required': [
          'relation'
        ],
        'properties': {
          'relation': {
            '$ref': 'http://dash.org/schemas/sys#/definitions/relation'
          }
        }
      },
      'profile': {
        'type': 'object',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        '_isrole': true,
        'required': [
          'address'
        ],
        'properties': {
          'name': {
            'type': 'string',
            'maxLength': 144,
            'minLength': 1
          },
          'text': {
            'type': 'string',
            'maxLength': 144,
            'minLength': 1
          },
          'address': {
            'type': 'string'
          },
          'avatarUrl': {
            'type': 'string',
            'format': 'uri'
          }
        }
      }
    },
    'HelloWorld': {
      '$id': 'http://dash.org/schema/dap',
      'oneOf': [
        {
          'required': [
            'someobject'
          ]
        }
      ],
      'title': 'HellowWorld',
      'someobject': {
        'type': 'object',
        'allOf': [
          {
            '$ref': 'http://dash.org/schemas/sys#/definitions/dapobjectbase'
          }
        ],
        'properties': {
          'helloworld': {
            'type': 'string'
          }
        }
      },
      'additionalProperties': false
    }
  }
};

const validAddressWithoutOutputs = 'yVWnW3MY3QHNXgptKg1iQuCkqmtFhMGyPF';
const invalidAddress = '123';

const validUsername = 'Alice';
const notExistingUsername = 'Bob';
const invalidUsername = '1.2';

const validBlockHash = '0000000005b3f97e0af8c72f9a96eca720237e374ca860938ba0d7a68471c4d6';
const validBaseBlockHash = '00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c';

const validBlockHeader =
  {
    'hash': '000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e',
    'confirmations': 6213,
    'height': 0,
    'version': 1,
    'versionHex': '00000001',
    'merkleroot': 'e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7',
    'time': 1417713337,
    'mediantime': 1417713337,
    'nonce': 1096447,
    'bits': '207fffff',
    'difficulty': 4.656542373906925e-10,
    'chainwork': '0000000000000000000000000000000000000000000000000000000000000002',
    'nextblockhash': '3f4a8012763b1d9b985cc77b0c0bca918830b1ef7dd083665bdc592c2cd31cf6'
  };

const validMnListDiff = SMNListFixture.getFirstDiff();

const dapObjects = [{
  'avatar': 'My avatar here',
  'aboutme': 'This is story about me',
  'objtype': 'user',
  'pver': null,
  'idx': 0,
  'rev': 0,
  'act': 0
}];

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

const transitionHash = '81d4247fbadf79acc937e21c4f877fae7442ac57403bbaff18cbaab45d4ff4ae';

const blocks = [{
  'height': 3689,
  'size': 616,
  'hash': '00000082bb900d7a37740e5642c20fa51a892743f46584ebfaf7f3d048086625',
  'time': 1545946963,
  'txlength': 2,
  'poolInfo': {}
}];

function validateUsername(uname) {
  return uname.length >= 3 && uname.length <= 12 && /^[\x00-\x7F]+$/.test('uname');
}

describe('api', () => {

  before(() => {
    // stub requests to DAPI

    sinon.stub(rpcClient, 'request')
      .callsFake(async function (url, method, params) {
          const {
            address, username, userId, rawTransaction, rawTransitionHeader, rawTransitionPacket, height, blockHash, baseBlockHash
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
              return validAddressSummary.balanceSat;
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
            if (!rawTransitionHeader) {
              throw new Error('Data packet is missing');
            }
            return transitionHash;
          }
          if (method === 'sendRawTransaction') {
            return {
              'txid': '9eda025a3b9e1e31e883f0cf2d249f4218466677c6707ec98b1f3f4a4570fa1a'
            };
          }
          if (method === 'getBestBlockHash') {
            return validBlockHash;
          }
          if (method === 'getBestBlockHeight') {
            return 100;
          }
          if (method === 'getBlockHash') {
            if (height === 0) {
              return validBaseBlockHash;
            }
            throw new Error('Invalid block height');
          }
          if (method === 'getMNList') {
            return [];
          }
          if (method === 'getMempoolInfo') {
            return {
              size: 0,
              bytes: 0,
              usage: 384,
              maxmempool: 300000000,
              mempoolminfee: 0.00000000,
            };
          }
          if (method === 'getBlockHeader') {
            if (blockHash === validBlockHash) {
              return validBlockHeader;
            }
            throw new Error('Invalid block hash');
          }
          if (method === 'getMnListDiff') {
            if (baseBlockHash === validBaseBlockHash || config.nullHash && blockHash === validBlockHash) {
              return validMnListDiff;
            }
            throw new Error('Invalid baseBlockHash or blockHash');
          }
          if (method === 'getAddressSummary') {
            return validAddressSummary;
          }
          if (method === 'estimateFee') {
            return { '2': 6.5e-7 };
          }
          if (method === 'getAddressUnconfirmedBalance') {
            return validAddressSummary.unconfirmedBalanceSat;
          }
          if (method === 'getAddressTotalReceived') {
            return validAddressSummary.totalReceivedSat;
          }
          if (method === 'getAddressTotalSent') {
            return validAddressSummary.totalSentSat;
          }
          if (method === 'getTransactionsByAddress') {
            return validAddressTransactions;
          }
          if (method === 'getTransactionById') {
            return validAddressTransactions.items[0];
          }
          if (method === 'getTransaction') {
            return validAddressTransactions.items[0];
          }
          if (method === 'getBlockHeaders') {
            return [validBlockHeader];
          }
          if (method === 'getBlocks') {
            return blocks;
          }
          if (method === 'getHistoricBlockchainDataSyncStatus') {
            return historicBlockchainDataSyncStatus;
          }
          if (method === 'estimateFee') {
            return {
              '2': 6.5e-7
            };
          }
          if (method === 'getRawBlock') {
            return rawBlock;
          }
          if (method === 'fetchDapContract') {
            return dapContract;
          }
          if (method === 'fetchDapObjects') {
            return dapObjects;
          }
          if (method === 'sendRawIxTransaction') {
            return {
              'txid': '9eda025a3b9e1e31e883f0cf2d249f4218466677c6707ec98b1f3f4a4570fa1a'
            };
          }
          if (method === 'searchUsers') {
            return {
              totalCount: 2,
              results: ['dash', 'dash2']
            };
          }
          if (method === 'getSpvData') {
            return {
              hash: validBlockHash
            };
          }
          if (method === 'loadBloomFilter') {
            return [];
          }
          if (method === 'addToBloomFilter') {
            return [];
          }
          if (method === 'clearBloomFilter') {
            return [];
          }
        }
      );
  });

  after(() => {
    // Restore stubbed DAPI request
    rpcClient.request.restore();
  });

  describe('constructor', () => {
    it('Should set seeds and port, if passed', async () => {
      const dapi = new Api({
        seeds: [{ service: '127.1.2.3:19999' }],
        port: 1234
      });
      expect(dapi.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.masternodeList).to.be.deep.equal([{ service: '127.1.2.3:19999' }]);
      expect(dapi.MNDiscovery.seeds).to.be.deep.equal([{ service: '127.1.2.3:19999' }]);

      await dapi.getBestBlockHash();
      //const baseHash = config.nullHash;
      const baseHash = validBaseBlockHash;
      const blockHash = validBlockHash;
      expect(rpcClient.request.calledWith({
        host: '127.1.2.3',
        port: 1234
      }, 'getMnListDiff', {
        baseBlockHash: baseHash,
        blockHash: blockHash
      })).to.be.true;
      expect(rpcClient.request.calledWith({
        host: '127.1.2.3',
        port: 1234
      }, 'getBestBlockHash', {})).to.be.true;
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
  describe('.address.getAddressSummary', () => {
    it('Should return a summary for an address', async () => {
      const dapi = new Api();
      const summary = await dapi.getAddressSummary(validAddressWithOutputs);
      expect(summary).to.be.an('object');
      expect(summary.balanceSat).to.be.a('number');
      expect(summary.unconfirmedBalanceSat).to.be.an('number');
      expect(summary.transactions).to.be.an('array');
      expect(summary.addrStr).to.be.equal(validAddressWithOutputs);
    });
    it('Should equal options.retries passed in', async () => {
      const options = { retries: 1 };
      const dapi = new Api(options);
      const summary = await dapi.getAddressSummary(validAddressWithOutputs);
      expect(dapi.retries).to.equal(1);
    });
  });
  describe('.address.getAddressUnconfirmedBalance', () => {
    it('Should return unconfirmed balance', async () => {
      const dapi = new Api();
      const unconfirmedBalance = await dapi.getAddressUnconfirmedBalance(validAddressWithOutputs);
      expect(unconfirmedBalance).to.be.a('number');
      expect(unconfirmedBalance).to.be.equal(validAddressSummary.unconfirmedBalanceSat);
    });
  });
  describe('.address.getAddressTotalReceived', () => {
    it('Should return total received value', async () => {
      const dapi = new Api();
      const totalReceived = await dapi.getAddressTotalReceived(validAddressWithOutputs);
      expect(totalReceived).to.be.a('number');
      expect(totalReceived).to.be.equal(validAddressSummary.totalReceivedSat);
    });
  });
  describe('.address.getAddressTotalSent', () => {
    it('Should return total sent value', async () => {
      const dapi = new Api();
      const totalReceived = await dapi.getAddressTotalSent(validAddressWithOutputs);
      expect(totalReceived).to.be.a('number');
      expect(totalReceived).to.be.equal(validAddressSummary.totalSentSat);
    });
  });
  describe('.tx.getTransaction', () => {
    it('Should get transaction', async () => {
      const dapi = new Api();
      const transaction = await dapi.getTransaction(validAddressTransactions.items[0].txid);
      expect(transaction).to.be.deep.equal(validAddressTransactions.items[0]);
    });
  });
  describe('.address.getTransactionsByAddress', () => {
    it('Should return transactions by address', async () => {
      const dapi = new Api();
      const summary = await dapi.getTransactionsByAddress(validAddressWithOutputs);
      expect(summary).to.be.deep.equal(validAddressTransactions);
    });
  });
  describe('.address.getTransactionById', () => {
    it('Should return transaction by id', async () => {
      const dapi = new Api();
      const summary = await dapi.getTransactionById(validAddressTransactions.items[0].txid);
      expect(summary).to.be.deep.equal(validAddressTransactions.items[0]);
    });
  });
  describe('.address.getBalance', () => {
    it('Should return sum of unspent outputs for address', async () => {
      const dapi = new Api();
      const balance = await dapi.getBalance(validAddressWithOutputs);
      expect(balance).to.be.equal(validAddressSummary.balanceSat);
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
      const user = await dapi.getUserByName(validUsername);
      dapi.generate(10);
      return expect(dapi.getUserById(user.regtxid + 'fake')).to.be.rejected;
    });
    it('Should throw error if user id not found', async () => {
      const dapi = new Api();
      return expect(dapi.getUserById(notExistingUsername)).to.be.rejected;
    });
    it('Should return user data if user exists', async () => {
      const dapi = new Api();
      const user = await dapi.getUserByName(validUsername);
      const userById = await dapi.getUserById(user.regtxid);
      expect(userById).to.be.an('object');
    });
  });
  describe('.user.searchUsers', () => {
    it('Should return users', async () => {
      const dapi = new Api();
      const pattern = '';
      const res = await dapi.searchUsers({
        pattern: 'Dash',
        offset: -1,
        limit: 10
      });
      expect(res).to.be.deep.equal({
          "results": [
            "dash",
            "dash2"
          ],
          "totalCount": 2
        });
    });
  });
  describe('.getSpvData', () => {
    it('Should return getSpvData', async () => {
      const dapi = new Api();
      const filter = '';
      const res = await dapi.getSpvData(filter);
      expect(res).to.be.deep.equal({
          "hash": validBlockHash
        });
    });
  });
  describe('.block.getBestBlockHash', () => {
    it('Should return chaintip hash', async () => {
      const dapi = new Api();
      const bestBlockHash = await dapi.getBestBlockHash();
      expect(bestBlockHash).to.be.a('string');
      expect(bestBlockHash).to.be.equal(validBlockHash);
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
      const blockHash = await dapi.getBlockHash(0);
      expect(blockHash).to.be.a('string');
      expect(blockHash).to.be.equal(validBaseBlockHash);
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

  describe('.block.getBlockHeader', () => {
    it('Should return block header by hash', async () => {
      const dapi = new Api();
      const blockHeader = await dapi.getBlockHeader(validBlockHash);
      expect(blockHeader.height).to.exist;
      expect(blockHeader.bits).to.exist;
      expect(blockHeader.chainwork).to.exist;
      expect(blockHeader.confirmations).to.exist;
      expect(blockHeader.difficulty).to.exist;
      expect(blockHeader.hash).to.exist;
      expect(blockHeader.mediantime).to.exist;
      expect(blockHeader.merkleroot).to.exist;
      expect(blockHeader.nextblockhash).to.exist;
      expect(blockHeader.nonce).to.exist;
      expect(blockHeader.time).to.exist;
      expect(blockHeader.version).to.exist;
    });
  });

  describe('.block.getBlockHeaders', () => {
    it('Should return block headers by hash', async () => {
      const dapi = new Api();
      const blockHeaders = await dapi.getBlockHeaders(2357, 3);
      expect(blockHeaders.length).to.be.equal(1);
      expect(blockHeaders[0].height).to.exist;
      expect(blockHeaders[0].bits).to.exist;
      expect(blockHeaders[0].chainwork).to.exist;
      expect(blockHeaders[0].confirmations).to.exist;
      expect(blockHeaders[0].difficulty).to.exist;
      expect(blockHeaders[0].hash).to.exist;
      expect(blockHeaders[0].mediantime).to.exist;
      expect(blockHeaders[0].merkleroot).to.exist;
      expect(blockHeaders[0].nextblockhash).to.exist;
      expect(blockHeaders[0].nonce).to.exist;
      expect(blockHeaders[0].time).to.exist;
      expect(blockHeaders[0].version).to.exist;
    });
  });

  describe('.block.getBlocks', () => {
    it('Should return blocks by blockDate and limit', async () => {
      const dapi = new Api();
      const blocks = await dapi.getBlocks('2018-12-24', 3);
      expect(blocks).to.be.deep.equal(blocks);
    });
  });

  describe('.block.getHistoricBlockchainDataSyncStatus', () => {
    it('Should return historic blockchain data sync status', async () => {
      const dapi = new Api();
      const dataSyncStatus = await dapi.getHistoricBlockchainDataSyncStatus();
      expect(dataSyncStatus).to.be.deep.equal(historicBlockchainDataSyncStatus);
    });
  });

  describe('.block.getRawBlock', () => {
    it('Should return raw block', async () => {
      const dapi = new Api();
      const getRawBlock = await dapi.getRawBlock();
      expect(getRawBlock).to.be.deep.equal(rawBlock);
    });
  });

  describe('.block.estimateFee', () => {
    it('Should return estimate fee', async () => {
      const dapi = new Api();
      const estimateFee = await dapi.estimateFee(2);
      expect(estimateFee).to.be.deep.equal({ '2': 6.5e-7 });
    });
  });

  describe('.tx.sendRawTransition', () => {
    xit('Should send raw transition', async () => {
      // 1. Create ST packet
      let { stpacket: stPacket } = Schema.create.stpacket();
      stPacket = Object.assign(stPacket, dapContract);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(validBlockchainUserObject.regtxid)
        .setHashPrevSubTx(validBlockchainUserObject.regtxid)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(new PrivateKey());

      const dapi = new Api();
      const transition = await dapi.sendRawTransition(transaction.serialize(),
        serializedPacket.toString('hex'),
      );
      expect(transition).to.be.deep.equal(transitionHash);
    });
    it('Should throw error when data packet is missing', async () => {
      const dapi = new Api();
      return expect(dapi.sendRawTransition()).to.be.rejected;
    });
  });

  describe('.tx.fetchDapContract', () => {
    it('Should fetch dap contract', async () => {
      const dapi = new Api();
      const dapContract = await dapi.fetchDapContract(dapId);
      expect(dapContract).to.be.deep.equal(dapContract);
    });
  });

  describe('.tx.fetchDapObjects', () => {
    it('Should fetch dap objects', async () => {
      const dapi = new Api();
      const dapContract = await dapi.fetchDapObjects(dapId, 'user', {});
      expect(dapContract).to.be.deep.equal(dapObjects);
    });
  });

  describe('.tx.sendRawTransaction', () => {
    it('Should return txid', async () => {
      const dapi = new Api();
      const rawTransaction = {};
      const tx = await dapi.sendRawTransaction(rawTransaction);
      // TODO: implement real unit test
      expect(tx.txid).to.be.deep.equal('9eda025a3b9e1e31e883f0cf2d249f4218466677c6707ec98b1f3f4a4570fa1a');
    });
  });

  describe('.tx.sendRawIxTransaction', () => {
    it('Should return txid', async () => {
      const dapi = new Api();
      const rawIxTransaction = {};
      const tx = await dapi.sendRawIxTransaction(rawIxTransaction);
      // TODO: implement real unit test
      expect(tx.txid).to.be.deep.equal('9eda025a3b9e1e31e883f0cf2d249f4218466677c6707ec98b1f3f4a4570fa1a');
    });
  });

  describe('.mnlist.getMnListDiff', () => {
    it('Should return mnlistdiff', async () => {
      const dapi = new Api();
      const mnlistdiff = await dapi.getMnListDiff(validBaseBlockHash, validBlockHash);
      expect(mnlistdiff.baseBlockHash).to.be.equal(validBaseBlockHash);
      expect(mnlistdiff.blockHash).to.be.equal(validBlockHash);
      expect(mnlistdiff.deletedMNs).to.be.an('array');
      expect(mnlistdiff.mnList).to.be.an('array');
    });
  });

  describe('.mempool.getMempoolInfo', () => {
    it('Should return mempool info', async () => {
      const dapi = new Api();
      const info = await dapi.getMempoolInfo();
      expect(info.size).to.be.equal(0);
      expect(info.bytes).to.be.equal(0);
      expect(info.usage).to.be.equal(384);
      expect(info.maxmempool).to.be.equal(300000000);
      expect(info.mempoolminfee).to.be.equal(0.00000000);
      expect(info.size).to.be.a('number');
      expect(info.bytes).to.be.a('number');
      expect(info.usage).to.be.a('number');
      expect(info.maxmempool).to.be.a('number');
      expect(info.mempoolminfee).to.be.a('number');
    });
  });

  describe('.spv.loadBloomFilter', () => {
    it('Should return loadBloomFilter', async () => {
      const dapi = new Api();
      const filter = '';
      const res = await dapi.loadBloomFilter(filter);
      // TODO: implement real unit test
      expect(res).to.be.deep.equal([]);
    });
  });

  describe('.spv.addToBloomFilter', () => {
    it('Should return addToBloomFilter', async () => {
      const dapi = new Api();
      const filter = '';
      const res = await dapi.addToBloomFilter(filter);
      // TODO: implement real unit test
      expect(res).to.be.deep.equal([]);
    });
  });

  describe('.spv.clearBloomFilter', () => {
    it('Should return clearBloomFilter', async () => {
      const dapi = new Api();
      const filter = '';
      const res = await dapi.clearBloomFilter(filter);
      // TODO: implement real unit test
      expect(res).to.be.deep.equal([]);
    });
  });
});
