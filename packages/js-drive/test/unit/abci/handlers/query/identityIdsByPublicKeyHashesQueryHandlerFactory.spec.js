const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const identityIdsByPublicKeyHashesQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identityIdsByPublicKeyHashesQueryHandlerFactory',
);
const InvalidArgumentAbciError = require(
  '../../../../../lib/abci/errors/InvalidArgumentAbciError',
);
const UnavailableAbciError = require('../../../../../lib/abci/errors/UnavailableAbciError');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

describe('identityIdsByPublicKeyHashesQueryHandlerFactory', () => {
  let identityIdsByPublicKeyHashesQueryHandler;
  let previousPublicKeyIdentityIdRepositoryMock;
  let publicKeyHashes;
  let identityIds;
  let maxIdentitiesPerRequest;
  let previousRootTreeMock;
  let previousPublicKeyToIdentityIdStoreRootTreeLeafMock;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextMock;
  let previousBlockExecutionContextMock;
  let params;
  let data;

  beforeEach(function beforeEach() {
    previousPublicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    maxIdentitiesPerRequest = 5;

    previousRootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    previousPublicKeyToIdentityIdStoreRootTreeLeafMock = this.sinon.stub();

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetIdentityIdsByPublicKeyHashesResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      previousPublicKeyIdentityIdRepositoryMock,
      maxIdentitiesPerRequest,
      previousRootTreeMock,
      previousPublicKeyToIdentityIdStoreRootTreeLeafMock,
      createQueryResponseMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
    );

    publicKeyHashes = [
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1328', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1329', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1330', 'hex'),
    ];

    identityIds = [
      generateRandomIdentifier(),
      generateRandomIdentifier(),
    ];

    previousPublicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[0])
      .resolves(identityIds[0]);

    previousPublicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[1])
      .resolves(identityIds[1]);

    params = {};
    data = { publicKeyHashes };
  });

  it('should return empty response if blockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    responseMock = new GetIdentityIdsByPublicKeyHashesResponse();
    responseMock.setIdentityIdsList([Buffer.alloc(0), Buffer.alloc(0), Buffer.alloc(0)]);
    responseMock.setMetadata(new ResponseMetadata());

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, {});

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch).to.have.not.been.called();
    expect(previousRootTreeMock.getFullProof).to.have.not.been.called();
  });

  it('should return empty response if previousBlockExecutionContext is empty', async () => {
    previousBlockExecutionContextMock.isEmpty.returns(true);

    responseMock = new GetIdentityIdsByPublicKeyHashesResponse();
    responseMock.setIdentityIdsList([Buffer.alloc(0), Buffer.alloc(0), Buffer.alloc(0)]);
    responseMock.setMetadata(new ResponseMetadata());

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, {});

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch).to.have.not.been.called();
    expect(previousRootTreeMock.getFullProof).to.have.not.been.called();
  });

  it('should throw an error if maximum requested items exceeded', async () => {
    maxIdentitiesPerRequest = 1;

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      previousPublicKeyIdentityIdRepositoryMock,
      maxIdentitiesPerRequest,
      previousRootTreeMock,
      previousPublicKeyToIdentityIdStoreRootTreeLeafMock,
      createQueryResponseMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
    );

    try {
      await identityIdsByPublicKeyHashesQueryHandler(params, data, {});
      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentAbciError);
      expect(e.getData()).to.deep.equal({
        maxIdentitiesPerRequest,
      });
    }
  });

  it('should return identity id map', async () => {
    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, {});

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should return identity id map with proof', async () => {
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    previousRootTreeMock.getFullProof.returns(proof);

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, { prove: 'true' });

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(previousPublicKeyIdentityIdRepositoryMock.fetch.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
    expect(previousRootTreeMock.getFullProof).to.be.calledOnce();
    expect(previousRootTreeMock.getFullProof.getCall(0).args).to.have.deep.members([
      previousPublicKeyToIdentityIdStoreRootTreeLeafMock,
      identityIds.map((identityId) => identityId.toBuffer()),
    ]);
  });

  it('should not proceed forward if createQueryResponse throws UnavailableAbciError', async () => {
    createQueryResponseMock.throws(new UnavailableAbciError());

    try {
      await identityIdsByPublicKeyHashesQueryHandler({}, {}, {});

      expect.fail('should throw UnavailableAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnavailableAbciError);
      expect(previousPublicKeyIdentityIdRepositoryMock.fetch).to.have.not.been.called();
    }
  });
});
