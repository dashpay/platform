const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const identitiesByPublicKeyHashesQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identitiesByPublicKeyHashesQueryHandlerFactory',
);
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const StorageResult = require('../../../../../lib/storage/StorageResult');

describe('identitiesByPublicKeyHashesQueryHandlerFactory', () => {
  let identitiesByPublicKeyHashesQueryHandler;
  let identityRepositoryMock;
  let publicKeyHashes;
  let identities;
  let maxIdentitiesPerRequest;
  let createQueryResponseMock;
  let responseMock;
  let params;
  let data;

  beforeEach(function beforeEach() {
    identityRepositoryMock = {
      fetchManyBuffers: this.sinon.stub(),
      proveMany: this.sinon.stub(),
    };

    maxIdentitiesPerRequest = 5;

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetIdentitiesByPublicKeyHashesResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    identitiesByPublicKeyHashesQueryHandler = identitiesByPublicKeyHashesQueryHandlerFactory(
      identityRepositoryMock,
      maxIdentitiesPerRequest,
      createQueryResponseMock,
    );

    publicKeyHashes = [
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1328', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1329', 'hex'),
      Buffer.from('784ca12495d2e61f992db9e55d1f9599b0cf1330', 'hex'),
    ];

    identities = [
      getIdentityFixture(),
      getIdentityFixture(),
    ];

    identityRepositoryMock
      .fetchManyBuffers.resolves(
        new StorageResult([identities[0].toBuffer(), identities[1].toBuffer()]),
      );

    params = {};
    data = { publicKeyHashes };
  });

  it('should throw an error if maximum requested items exceeded', async () => {
    maxIdentitiesPerRequest = 1;

    identitiesByPublicKeyHashesQueryHandler = identitiesByPublicKeyHashesQueryHandlerFactory(
      identityRepositoryMock,
      maxIdentitiesPerRequest,
      createQueryResponseMock,
    );

    try {
      await identitiesByPublicKeyHashesQueryHandler(params, data, {});

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentAbciError);
      expect(e.getData()).to.deep.equal({
        maxIdentitiesPerRequest,
      });
    }
  });

  it('should return identities', async () => {
    params = publicKeyHashes;

    const result = await identitiesByPublicKeyHashesQueryHandler(params, data, {});

    expect(identityRepositoryMock.fetchManyBuffers).to.be.calledOnceWithExactly(
      publicKeyHashes,
    );

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should return proof if it was requested', async () => {
    // const proof = {
    //   rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
    //   'hex'),
    //   storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
    //   'hex'),
    // };

    const proof = Buffer.alloc(20, 1);

    identityRepositoryMock.proveMany.resolves(
      new StorageResult(proof),
    );

    const result = await identitiesByPublicKeyHashesQueryHandler(params, data, { prove: true });

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());

    expect(identityRepositoryMock.proveMany).to.be.calledOnceWithExactly(
      data.publicKeyHashes,
    );
  });
});
