const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetBlockResponse,
  },
} = require('@dashevo/dapi-grpc');

const { Block } = require('@dashevo/dashcore-lib');

const getBlockHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getBlockHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getBlockHandlerFactory', () => {
  let call;
  let hash;
  let height;
  let getBlockHandler;
  let coreRPCClientMock;
  let request;
  let block;

  beforeEach(function beforeEach() {
    hash = '';
    height = 0;

    const serializedBlock = '02000000b67a40f3cd5804437a108f105533739c37e6229bc1adcab385140b59fd0f0000a71c1aade44bf8425bec0deb611c20b16da3442818ef20489ca1e2512be43eef814cdb52f0ff0f1edbf701000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0a510101062f503253482fffffffff0100743ba40b000000232103a69850243c993c0645a6e8b38c774174174cc766cd3ec2140afd24d831b84c41ac00000000';

    block = new Block(Buffer.from(serializedBlock, 'hex'));
    request = {
      getHeight: this.sinon.stub().returns(height),
      getHash: this.sinon.stub().returns(hash),
    };

    call = new GrpcCallMock(this.sinon, request);

    coreRPCClientMock = {
      getRawBlock: this.sinon.stub().resolves(serializedBlock),
      getBlockHash: this.sinon.stub().resolves(hash),
    };

    getBlockHandler = getBlockHandlerFactory(coreRPCClientMock);
  });

  it('should return valid result is hash is specified', async () => {
    hash = 'hash';
    request.getHash.returns(hash);

    const result = await getBlockHandler(call);

    expect(result).to.be.an.instanceOf(GetBlockResponse);

    expect(coreRPCClientMock.getRawBlock).to.be.calledOnceWith(hash);
    expect(coreRPCClientMock.getBlockHash).to.be.not.called();

    const blockBinary = result.getBlock();

    expect(blockBinary).to.be.an.instanceOf(Buffer);

    const returnedBlock = new Block(blockBinary);

    expect(returnedBlock.toJSON()).to.deep.equal(block.toJSON());
  });

  it('should return valid result is height is specified', async () => {
    height = 42;
    request.getHeight.returns(height);

    const result = await getBlockHandler(call);

    expect(result).to.be.an.instanceOf(GetBlockResponse);

    expect(coreRPCClientMock.getRawBlock).to.be.called();
    expect(coreRPCClientMock.getBlockHash).to.be.calledOnceWith(height);

    const blockBinary = result.getBlock();

    expect(blockBinary).to.be.an.instanceOf(Buffer);

    const returnedBlock = new Block(blockBinary);

    expect(returnedBlock.toJSON()).to.deep.equal(block.toJSON());
  });

  it('should throw an InvalidArgumentGrpcError if hash and height are not specified', async () => {
    try {
      await getBlockHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('hash or height is not specified');
      expect(coreRPCClientMock.getRawBlock).to.be.not.called();
      expect(coreRPCClientMock.getBlockHash).to.be.not.called();
    }
  });
  it('should throw an InvalidArgumentGrpcError if getRawBlock throws error with code -1', async () => {
    const error = new Error('JSON value is not an integer as expected');
    error.code = -1;

    coreRPCClientMock.getBlockHash.throws(error);

    height = 'abc';
    request.getHeight.returns(height);

    try {
      await getBlockHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('JSON value is not an integer as expected');
      expect(coreRPCClientMock.getBlockHash).to.be.calledOnceWith(height);
    }
  });

  it('should throw an NotFoundGrpcError if block with specified height is not found', async () => {
    const error = new Error('Block height out of range');
    error.code = -8;

    coreRPCClientMock.getBlockHash.throws(error);

    height = 111111111;
    request.getHeight.returns(height);

    try {
      await getBlockHandler(call);

      expect.fail('should thrown NotFoundGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.getMessage()).to.equal('Invalid block height');
      expect(coreRPCClientMock.getBlockHash).to.be.calledOnceWith(height);
    }
  });

  it('should throw an InvalidArgumentGrpcError if getRawBlock throws error with code -5', async () => {
    const error = new Error();
    error.code = -5;

    coreRPCClientMock.getRawBlock.throws(error);

    hash = 'hash';
    request.getHash.returns(hash);

    try {
      await getBlockHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.getMessage()).to.equal('Block not found');
      expect(coreRPCClientMock.getBlockHash).to.be.not.called();
      expect(coreRPCClientMock.getRawBlock).to.be.calledOnceWith(hash);
    }
  });

  it('should throw an InternalGrpcError if getRawBlockByHash throws unknown error', async () => {
    const error = new Error('Unknown error');

    coreRPCClientMock.getRawBlock.throws(error);

    hash = 'hash';
    request.getHash.returns(hash);

    try {
      await getBlockHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(coreRPCClientMock.getBlockHash).to.be.not.called();
      expect(coreRPCClientMock.getRawBlock).to.be.calledOnceWith(hash);
    }
  });
});
