const { EventEmitter } = require('events');

const {
  v0: {
    BroadcastTransactionResponse,
    GetBlockResponse,
    GetTransactionResponse,
    GetBlockchainStatusResponse,
    GetMasternodeStatusResponse,
  },
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

  describe('#getBlockchainStatus', () => {
    it('should get status', async () => {
      const response = new GetBlockchainStatusResponse();

      response.setStatus(GetBlockchainStatusResponse.Status.READY);

      grpcTransportMock.request.resolves(response);

      await coreMethods.getBlockchainStatus();

      expect(jsonRpcTransportMock.request).to.be.not.called();
      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getMasternodeStatus', () => {
    it('should get masternode status', async () => {
      const response = new GetMasternodeStatusResponse();

      response.setStatus(GetMasternodeStatusResponse.Status.READY);

      grpcTransportMock.request.resolves(response);

      await coreMethods.getMasternodeStatus();

      expect(jsonRpcTransportMock.request).to.be.not.called();
      expect(grpcTransportMock.request).to.be.calledOnce();
    });
  });

  describe('#getTransaction', () => {
    it('should get transaction', async () => {
      const transaction = Buffer.from('transaction');
      const response = new GetTransactionResponse();
      response.setTransaction(transaction);
      response.setBlockHash(Buffer.from('blockHash'));
      response.setHeight(1);
      response.setConfirmations(2);

      grpcTransportMock.request.resolves(response);
      await coreMethods.getTransaction('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176');

      expect(grpcTransportMock.request).to.be.calledOnce();
      expect(jsonRpcTransportMock.request).to.be.not.called();
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
