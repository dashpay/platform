const {
  v0: {
    GetTransactionResponse: ProtoGetTransactionResponse,
  },
} = require('@dashevo/dapi-grpc');
const GetTransactionResponse = require('../../../../../lib/methods/core/getTransaction/GetTransactionResponse');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');

describe('GetTransactionResponse', () => {
  let getTransactionResponse;
  let response;
  let proto;

  beforeEach(() => {
    response = {
      transaction: Buffer.from('transaction'),
      blockHash: Buffer.from('blockHash'),
      height: 10,
      confirmations: 42,
      instantLocked: true,
      chainLocked: false,
    };

    proto = new ProtoGetTransactionResponse();
    proto.setTransaction(response.transaction);
    proto.setBlockHash(response.blockHash);
    proto.setHeight(response.height);
    proto.setConfirmations(response.confirmations);
    proto.setIsChainLocked(response.isChainLocked);
    proto.setIsInstantLocked(response.isInstantLocked);

    getTransactionResponse = new GetTransactionResponse(response);
  });

  it('should return transaction', () => {
    const transaction = getTransactionResponse.getTransaction();

    expect(transaction).to.deep.equal(response.transaction);
  });

  it('should return block hash', () => {
    const blockHash = getTransactionResponse.getBlockHash();

    expect(blockHash).to.deep.equal(response.blockHash);
  });

  it('should return height', () => {
    const height = getTransactionResponse.getHeight();

    expect(height).to.deep.equal(response.height);
  });

  it('should return confirmations', () => {
    const confirmations = getTransactionResponse.getConfirmations();

    expect(confirmations).to.deep.equal(response.confirmations);
  });

  it('should return is transaction instantLocked', () => {
    const isInstantLocked = getTransactionResponse.isInstantLocked();

    expect(isInstantLocked).to.deep.equal(response.isInstantLocked);
  });

  it('should return is transaction chainLocked', () => {
    const isChainLocked = getTransactionResponse.isChainLocked();

    expect(isChainLocked).to.equal(response.isChainLocked);
  });

  it('should create an instance from proto', () => {
    const instance = GetTransactionResponse.createFromProto(proto);

    expect(instance).to.be.an.instanceOf(GetTransactionResponse);
    expect(instance.transaction).to.deep.equal(Buffer.from(proto.getTransaction()));
    expect(instance.blockHash).to.deep.equal(Buffer.from(proto.getBlockHash()));
    expect(instance.height).to.deep.equal(proto.getHeight());
    expect(instance.confirmations).to.deep.equal(proto.getConfirmations());
    expect(instance.instantLocked).to.deep.equal(proto.getIsInstantLocked());
    expect(instance.chainLocked).to.deep.equal(proto.getIsChainLocked());
  });

  it('should throw InvalidResponseError if Transaction is not defined', () => {
    proto.setTransaction(undefined);

    try {
      GetTransactionResponse.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if BlockHash is not defined', () => {
    proto.setBlockHash(undefined);

    try {
      GetTransactionResponse.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
