const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const cbor = require('cbor');

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

const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');
const UnimplementedAbciError = require('../../../../../lib/abci/errors/UnimplementedAbciError');

describe('identityIdsByPublicKeyHashesQueryHandlerFactory', () => {
  let identityIdsByPublicKeyHashesQueryHandler;
  let signedPublicKeyToIdentityIdRepository;
  let publicKeyHashes;
  let identityIds;
  let maxIdentitiesPerRequest;
  let createQueryResponseMock;
  let responseMock;
  let blockExecutionContextStackMock;
  let params;
  let data;

  beforeEach(function beforeEach() {
    signedPublicKeyToIdentityIdRepository = {
      fetchBuffer: this.sinon.stub(),
    };

    maxIdentitiesPerRequest = 5;

    createQueryResponseMock = this.sinon.stub();

    responseMock = new GetIdentityIdsByPublicKeyHashesResponse();
    responseMock.setProof(new Proof());

    createQueryResponseMock.returns(responseMock);

    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackMock.getLast.returns(true);

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      signedPublicKeyToIdentityIdRepository,
      maxIdentitiesPerRequest,
      createQueryResponseMock,
      blockExecutionContextStackMock,
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

    signedPublicKeyToIdentityIdRepository
      .fetchBuffer
      .withArgs(publicKeyHashes[0])
      .resolves(cbor.encode([identityIds[0]]));

    signedPublicKeyToIdentityIdRepository
      .fetchBuffer
      .withArgs(publicKeyHashes[1])
      .resolves(cbor.encode([identityIds[1]]));

    params = {};
    data = { publicKeyHashes };
  });

  it('should return empty response if there is no signed state', async () => {
    blockExecutionContextStackMock.getLast.returns(null);

    responseMock = new GetIdentityIdsByPublicKeyHashesResponse();
    responseMock.setIdentityIdsList([
      cbor.encode([]),
      cbor.encode([]),
      cbor.encode([]),
    ]);
    responseMock.setMetadata(new ResponseMetadata());

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, {});

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(result.value).to.deep.equal(responseMock.serializeBinary());

    expect(signedPublicKeyToIdentityIdRepository.fetchBuffer).to.have.not.been.called();
  });

  it('should throw an error if maximum requested items exceeded', async () => {
    maxIdentitiesPerRequest = 1;

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      signedPublicKeyToIdentityIdRepository,
      maxIdentitiesPerRequest,
      createQueryResponseMock,
      blockExecutionContextStackMock,
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

    expect(signedPublicKeyToIdentityIdRepository.fetchBuffer.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(signedPublicKeyToIdentityIdRepository.fetchBuffer.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(signedPublicKeyToIdentityIdRepository.fetchBuffer.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(signedPublicKeyToIdentityIdRepository.fetchBuffer.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(responseMock.serializeBinary());
  });

  it('should throw UnimplementedAbciError if proof requested', async () => {
    // const proof = {
    //   rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101',
    //   'hex'),
    //   storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210',
    //   'hex'),
    // };

    try {
      await identityIdsByPublicKeyHashesQueryHandler(params, data, { prove: true });

      expect.fail('should throw UnimplementedAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(UnimplementedAbciError);
    }
  });
});
