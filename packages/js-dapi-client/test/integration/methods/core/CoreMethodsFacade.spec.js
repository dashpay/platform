const { EventEmitter } = require('events');

const {
  BroadcastTransactionResponse,
  GetBlockResponse,
  GetTransactionResponse,
  GetStatusResponse,
} = require('@dashevo/dapi-grpc');

const BloomFilter = require('@dashevo/dashcore-lib/lib/bloomfilter');

const CoreMethodsFacade = require('../../../../lib/methods/core/CoreMethodsFacade');

describe('CoreMethodsFacade', () => {
  let jsonRpcTransportMock;
  let grpcTransportMock;
  let coreMethods;

  beforeEach(function beforeEach() {
    jsonRpcTransportMock = {
      request: this.sinon.stub(),
    };
    grpcTransportMock = {
      request: this.sinon.stub(),
    };

    coreMethods = new CoreMethodsFacade(jsonRpcTransportMock, grpcTransportMock);
  });

  describe('#broadcastTransaction', () => {
    it('should broadcast transaction', async () => {
      const response = new BroadcastTransactionResponse();
      response.setTransactionId('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176');
      grpcTransportMock.request.resolves(response);

      const transaction = Buffer.from('transaction');
      await coreMethods.broadcastTransaction(transaction);

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
    });
  });

  describe('#generateToAddress', () => {
    it('should generate address', async () => {
      const response = 'response';
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.generateToAddress(1, 'yTMDce5yEpiPqmgPrPmTj7yAmQPJERUSVy');

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getAddressSummary', () => {
    it('should get address summary', async () => {
      const response = {
        addrStr: 'yXdxAYfK8eJgQmHpUzMaKEBhqwKQWKSezS',
        balance: 4173964.74940914,
        balanceSat: 417396474940914,
        totalReceived: 4287576.24940914,
        totalReceivedSat: 428757624940914,
        totalSent: 113611.5,
        totalSentSat: 11361150000000,
        unconfirmedBalance: 0,
        unconfirmedBalanceSat: 0,
        unconfirmedTxApperances: 0,
        txApperances: 27434,
        transactions: ['4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', '8890a0ee95a17f6723ab2d9a0bdd579351b9220738ad34f5b49cbe63f09b082a'],
      };
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.getAddressSummary('yTMDce5yEpiPqmgPrPmTj7yAmQPJERUSVy');

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getBestBlockHash', () => {
    it('should get best block hash', async () => {
      const response = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.getBestBlockHash();

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getBlockByHash', () => {
    it('should get block by hash', async () => {
      const block = Buffer.from('block');
      const response = new GetBlockResponse();
      response.setBlock(block);
      grpcTransportMock.request.resolves(response);
      await coreMethods.getBlockByHash('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176');

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
    });
  });

  describe('#getBlockByHeight', () => {
    it('should get block by height', async () => {
      const block = Buffer.from('block');
      const response = new GetBlockResponse();
      response.setBlock(block);
      grpcTransportMock.request.resolves(response);
      await coreMethods.getBlockByHeight(1);

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
    });
  });

  describe('#getBlockHash', () => {
    it('should get block hash', async () => {
      const response = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.getBlockHash(1);

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getMnListDiff', () => {
    it('should get mn list diff', async () => {
      const baseBlockHash = '0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1';
      const blockHash = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';

      const response = {
        baseBlockHash,
        blockHash,
        deletedMNs: [],
        mnList: [],
      };
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.getMnListDiff(baseBlockHash, blockHash);

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getStatus', () => {
    it('should get status', async () => {
      const response = new GetStatusResponse();
      grpcTransportMock.request.resolves(response);
      await coreMethods.getStatus();

      expect(jsonRpcTransportMock.request).to.be.not.called();
      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getTransaction', () => {
    it('should get transaction', async () => {
      const transaction = Buffer.from('transaction');
      const response = new GetTransactionResponse();
      response.setTransaction(transaction);
      grpcTransportMock.request.resolves(response);
      await coreMethods.getTransaction('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176');

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
    });
  });

  describe('#getUTXO', () => {
    it('should get UTXO', async () => {
      const response = [{
        address: 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh',
        txid: '42c56c8ec8e2cdf97a56bf6290c43811ff59181a184b70ebbe3cb66b2970ced2',
        vout: 1,
        scriptPubKey: '76a914dc2bfda564dc6217c55c842d65cc0242e095d2d788ac',
        amount: 999.81530001,
        satoshis: 99981530001,
        confirmations: 0,
        ts: 1529847344,
      }];
      jsonRpcTransportMock.request.resolves(response);
      await coreMethods.getUTXO('yTMDce5yEpiPqmgPrPmTj7yAmQPJERUSVy');

      expect(grpcTransportMock.request).to.be.not.called();
      expect(jsonRpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#subscribeToTransactionsWithProofs', () => {
    it('should subscribe to transactions with proofs', async () => {
      const bloomFilter = BloomFilter.create(1, 0.001);
      const response = new EventEmitter();
      grpcTransportMock.request.resolves(response);
      await coreMethods.subscribeToTransactionsWithProofs(bloomFilter);

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
    });
  });
});
