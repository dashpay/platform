const sinon = require('sinon');
const {
  CorePromiseClient,
  PlatformPromiseClient,
  TransactionsFilterStreamPromiseClient,
  TransactionsWithProofsRequest,
  ApplyStateTransitionResponse,
  GetIdentityResponse,
  GetDocumentsResponse,
  GetDataContractResponse,
  GetBlockRequest,
  GetBlockResponse,
  GetStatusRequest,
  GetStatusResponse,
  GetTransactionRequest,
  GetTransactionResponse,
  SendTransactionRequest,
  SendTransactionResponse,
} = require('@dashevo/dapi-grpc');
const chai = require('chai');
const { EventEmitter } = require('events');
const DAPIClient = require('../../src/index');
const chaiAsPromised = require('chai-as-promised');
const sinonChai = require('sinon-chai');
const rpcClient = require('../../src/RPCClient');
const config = require('../../src/config');
const SMNListFixture = require('../fixtures/mnList');

const RPCError = require("../../src/errors/RPCError");

const {
  BloomFilter,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

chai.use(chaiAsPromised);
chai.use(sinonChai);

const { expect } = chai;

const validAddressWithOutputs = 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS';
const contractId = '11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c';

const contract = {
  'dapname': 'TestContacts_test',
  'dapver': 1,
  'idx': 0,
  'meta': {
    'id': contractId
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

const validBlockHash = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';
const validBaseBlockHash = '0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1';

const validMnListDiff = SMNListFixture.getFirstDiff();

const documents = [{
  'avatar': 'My avatar here',
  'aboutme': 'This is story about me',
  'objtype': 'user',
  'pver': null,
  'idx': 0,
  'rev': 0,
  'act': 0
}];

describe('api', () => {

  before(() => {
    // stub requests to DAPI

    sinon.stub(rpcClient, 'request')
      .callsFake(async function (url, method, params) {
          const {
            address, height, blockHash, baseBlockHash
          } = params;
          if (method === 'getUTXO') {
            if (address === validAddressWithOutputs) {
              return [{}];
            }
            if (address === validAddressWithoutOutputs) {
              return [];
            }
            throw new RPCError('DAPI RPC error: getBlockHash: Error: Address not found');
          }
          if (method === 'getBestBlockHash') {
            return validBlockHash;
          }
          if (method === 'getBlockHash') {
            if (height === 0) {
              return validBaseBlockHash;
            }
            throw new RPCError('DAPI RPC error: getBlockHash: Error: Invalid block height');
          }
          if (method === 'getMnListDiff') {
            if (baseBlockHash === validBaseBlockHash || config.nullHash && blockHash === validBlockHash) {
              return validMnListDiff;
            }
            throw new Error('Invalid baseBlockHash or blockHash');
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
      const dapi = new DAPIClient({
        seeds: [{ service: '127.1.2.3:19999' }],
        port: 1234
      });
      expect(dapi.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.DAPIPort).to.be.equal(1234);
      expect(dapi.MNDiscovery.masternodeListProvider.seeds).to.be.deep.equal([{ service: '127.1.2.3:19999' }]);
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
      const dapi = new DAPIClient();
      const utxo = await dapi.getUTXO(validAddressWithOutputs);
      expect(utxo).to.be.an('array');
      const output = utxo[0];
      expect(output).to.be.an('object');
    });
    it('Should return empty list if there is no unspent output', async () => {
      const dapi = new DAPIClient();
      const utxo = await dapi.getUTXO(validAddressWithoutOutputs);
      expect(utxo).to.be.an('array');
      expect(utxo.length).to.be.equal(0);
    });
    it('Should throw error if address is invalid/not found', async () => {
      const dapi = new DAPIClient();
      await expect(dapi.getUTXO(invalidAddress)).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Address not found');
    });
  });
  describe('.block.getBestBlockHash', () => {
    it('Should return chaintip hash', async () => {
      const dapi = new DAPIClient();
      const bestBlockHash = await dapi.getBestBlockHash();
      expect(bestBlockHash).to.be.a('string');
      expect(bestBlockHash).to.be.equal(validBlockHash);
    });
  });

  describe('.block.getBlockHash', () => {
    it('Should return hash for a given block height', async () => {
      const dapi = new DAPIClient();
      const blockHash = await dapi.getBlockHash(0);
      expect(blockHash).to.be.a('string');
      expect(blockHash).to.be.equal(validBaseBlockHash);
    });
    it('Should be rejected if height is invalid', async () => {
      const dapi = new DAPIClient();
      await expect(dapi.getBlockHash(1000000)).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Invalid block height');
      await expect(dapi.getBlockHash('some string')).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Invalid block height');
      await expect(dapi.getBlockHash(1.2)).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Invalid block height');
      await expect(dapi.getBlockHash(-1)).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Invalid block height');
      await expect(dapi.getBlockHash(true)).to.be.rejectedWith(RPCError, 'DAPI RPC error: getBlockHash: Error: Invalid block height');
    });
  });

  describe('.mnlist.getMnListDiff', () => {
    it('Should return mnlistdiff', async () => {
      const dapi = new DAPIClient();
      const mnlistdiff = await dapi.getMnListDiff(validBaseBlockHash, validBlockHash);
      expect(mnlistdiff.baseBlockHash).to.be.equal(validBaseBlockHash);
      expect(mnlistdiff.blockHash).to.be.equal(validBlockHash);
      expect(mnlistdiff.deletedMNs).to.be.an('array');
      expect(mnlistdiff.mnList).to.be.an('array');
    });
  });

  describe('#getBlockByHeight', () => {
    let getBlockStub;
    let height;

    beforeEach(() => {
      getBlockStub = sinon
        .stub(CorePromiseClient.prototype, 'getBlock');

      height = 1;
    });

    afterEach(() => {
      getBlockStub.restore();
    });

    it('should return a block as Buffer', async () => {
      const response = new GetBlockResponse();
      response.setBlock(Buffer.from('block'));
      getBlockStub.resolves(response);

      const request = new GetBlockRequest();
      request.setHeight(height);

      const client = new DAPIClient();
      const result = await client.getBlockByHeight(height);

      expect(getBlockStub).to.be.calledOnceWithExactly(request);

      expect(result).to.be.instanceof(Buffer);
    });
  });

  describe('#getBlockByHash', () => {
    let getBlockStub;
    let hash;

    beforeEach(() => {
      getBlockStub = sinon
        .stub(CorePromiseClient.prototype, 'getBlock');

      hash = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';
    });

    afterEach(() => {
      getBlockStub.restore();
    });

    it('should return a block as Buffer', async () => {
      const response = new GetBlockResponse();
      response.setBlock(Buffer.from('block'));
      getBlockStub.resolves(response);

      const request = new GetBlockRequest();
      request.setHash(hash);

      const client = new DAPIClient();
      const result = await client.getBlockByHash(hash);

      expect(getBlockStub).to.be.calledOnceWithExactly(request);

      expect(result).to.be.instanceof(Buffer);
    });
  });

  describe('#getStatus', () => {
    let getStatusStub;

    beforeEach(() => {
      getStatusStub = sinon
        .stub(CorePromiseClient.prototype, 'getStatus');
    });

    afterEach(() => {
      getStatusStub.restore();
    });

    it('should return status as plain object', async () => {
      const status = {
        coreVersion: 1,
        protocolVersion: 2,
        blocks: 3,
        timeOffset: 4,
        connections: 5,
        proxy: 'proxy',
        difficulty: 0.4344343,
        testnet: true,
        relayFee: 0.1321321,
        errors: '',
        network: 'mainnet',
      };

      const response = new GetStatusResponse();
      response.setCoreVersion(status.coreVersion);
      response.setProtocolVersion(status.protocolVersion);
      response.setBlocks(status.blocks);
      response.setTimeOffset(status.timeOffset);
      response.setConnections(status.connections);
      response.setProxy(status.proxy);
      response.setDifficulty(status.difficulty);
      response.setTestnet(status.testnet);
      response.setRelayFee(status.relayFee);
      response.setErrors(status.errors);
      response.setNetwork(status.network);

      getStatusStub.resolves(response);

      const request = new GetStatusRequest();

      const client = new DAPIClient();
      const result = await client.getStatus();

      expect(getStatusStub).to.be.calledOnceWithExactly(request);

      expect(result).to.be.deep.equal(status);
    });
  });

  describe('#getTransaction', () => {
    let getTransactionStub;
    let id;

    beforeEach(() => {
      getTransactionStub = sinon
        .stub(CorePromiseClient.prototype, 'getTransaction');

      id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';
    });

    afterEach(() => {
      getTransactionStub.restore();
    });

    it('should return a transaction as Buffer', async () => {
      const response = new GetTransactionResponse();
      response.setTransaction(Buffer.from('transaction'));
      getTransactionStub.resolves(response);

      const request = new GetTransactionRequest();
      request.setId(id);

      const client = new DAPIClient();
      const result = await client.getTransaction(id);

      expect(getTransactionStub).to.be.calledOnceWithExactly(request);

      expect(result).to.be.instanceof(Buffer);
    });
  });

  describe('#sendTransaction', () => {
    let sendTransactionStub;
    let transaction;
    let id;

    beforeEach(() => {
      sendTransactionStub = sinon
        .stub(CorePromiseClient.prototype, 'sendTransaction');

      transaction = Buffer.from('transaction');
      id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';
    });

    afterEach(() => {
      sendTransactionStub.restore();
    });

    it('should return a transaction as Buffer', async () => {
      const response = new SendTransactionResponse();
      response.setTransactionId(id);
      sendTransactionStub.resolves(response);

      const request = new SendTransactionRequest();
      request.setTransaction(transaction);
      request.setAllowHighFees(true);
      request.setBypassLimits(false);

      const client = new DAPIClient();
      const result = await client.sendTransaction(transaction, { allowHighFees: true });

      expect(sendTransactionStub).to.be.calledOnceWithExactly(request);

      expect(result).to.equal(id);
    });
  });

  describe('#subscribeToTransactionsWithProofs', () => {
    let stream;
    let grpcClientSubscribeMock;
    beforeEach(() => {
      stream = new EventEmitter();
      grpcClientSubscribeMock = sinon
        .stub(TransactionsFilterStreamPromiseClient.prototype, 'subscribeToTransactionsWithProofs')
        .returns(stream);
    });

    afterEach(() => {
      grpcClientSubscribeMock.restore();
    });

    it('should return a stream', async () => {
      const client = new DAPIClient();

      const fromBlockHeight = 1;
      const count = 2;
      const vData = Buffer.from([1]);

      const bloomFilter = BloomFilter.create(1, 0.001);

      const actualStream = await client.subscribeToTransactionsWithProofs(
        bloomFilter,
        { fromBlockHeight, count }
      );

      expect(grpcClientSubscribeMock.callCount).to.equal(1);

      const request = grpcClientSubscribeMock.getCall(0).args[0];

      expect(request).to.be.an.instanceOf(TransactionsWithProofsRequest);

      const actualBloomFilter = request.getBloomFilter();

      expect(actualBloomFilter.toObject()).to.be.deep.equal(
        Object.assign(bloomFilter.toObject(), {
          vData: Buffer.from(
            new Uint8Array(bloomFilter.toObject().vData),
          ).toString('base64'),
        }),
      );

      expect(request.getFromBlockHash()).to.equal('');
      expect(request.getFromBlockHeight()).to.equal(fromBlockHeight);
      expect(request.getCount()).to.equal(count);

      expect(actualStream).to.be.equal(stream);
    });
  });

  describe('#applyStateTransition', () => {
    let applyStateTransitionStub;
    let stateTransitionFixture;

    beforeEach(() => {
      userId = Buffer.alloc(256).toString('hex');

      applyStateTransitionStub = sinon
          .stub(PlatformPromiseClient.prototype, 'applyStateTransition');

      const dataContractFixture = getDataContractFixture();
      const dpp = new DashPlatformProtocol();

      stateTransitionFixture = dpp.dataContract.createStateTransition(dataContractFixture);
    });

    afterEach(() => {
      applyStateTransitionStub.restore();
    });

    it('should return ApplyStateTransitionResponse', async () => {
      const response = new ApplyStateTransitionResponse();
      applyStateTransitionStub.resolves(response);

      const client = new DAPIClient();
      const result = await client.applyStateTransition(stateTransitionFixture);

      expect(result).to.be.instanceOf(ApplyStateTransitionResponse);
    });
  });

  describe('#getIdentity', () => {
    let getIdentityStub;
    let id;

    beforeEach(() => {
      getIdentityStub = sinon
        .stub(PlatformPromiseClient.prototype, 'getIdentity');

      id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';
    });

    afterEach(() => {
      getIdentityStub.restore();
    });

    it('should return Buffer', async () => {
      const response = new GetIdentityResponse();
      response.setIdentity(Buffer.from('identity'));
      getIdentityStub.resolves(response);

      const client = new DAPIClient();
      const result = await client.getIdentity(id);

      expect(result).to.be.instanceof(Buffer);
    });
  });

  describe('#getDocuments', () => {
    let getDocumentsStub;
    let serializedDocuments;

    beforeEach(() => {
      serializedDocuments = documents.map(document => Buffer.from(JSON.stringify(document)));
      getDocumentsStub = sinon
        .stub(PlatformPromiseClient.prototype, 'getDocuments');
    });

    afterEach(() => {
      getDocumentsStub.restore();
    });

    it('should return documents', async () => {
      const response = new GetDocumentsResponse();
      response.setDocumentsList(serializedDocuments);
      getDocumentsStub.resolves(response);

      const client = new DAPIClient();
      const result = await client.getDocuments(contractId, 'user', {});

      expect(result).to.be.an('array');
      expect(result).to.be.deep.equal(serializedDocuments);
    });
  });

  describe('#getDataContract', () => {
    let getDataContractStub;
    let serializedDataContract;

    beforeEach(() => {
      serializedDataContract = Buffer.from(JSON.stringify(contract));

      getDataContractStub = sinon
        .stub(PlatformPromiseClient.prototype, 'getDataContract');
    });

    afterEach(() => {
      getDataContractStub.restore();
    });

    it('should return data contract', async () => {
      const response = new GetDataContractResponse();
      response.setDataContract(serializedDataContract);
      getDataContractStub.resolves(response);

      const client = new DAPIClient();
      const result = await client.getDataContract(contractId);

      expect(result).to.be.instanceof(Buffer);
      expect(result).to.be.deep.equal(serializedDataContract);
    });
  });
});
