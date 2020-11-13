const cbor = require('cbor');

const {
  abci: {
    ResponseQuery,
  },
} = require('abci/types');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const identityIdsByPublicKeyHashesQueryHandlerFactory = require(
  '../../../../../lib/abci/handlers/query/identityIdsByPublicKeyHashesQueryHandlerFactory',
);
const InvalidArgumentAbciError = require(
  '../../../../../lib/abci/errors/InvalidArgumentAbciError',
);

describe('identityIdsByPublicKeyHashesQueryHandlerFactory', () => {
  let identityIdsByPublicKeyHashesQueryHandler;
  let publicKeyIdentityIdRepositoryMock;
  let publicKeyHashes;
  let identityIds;
  let identityIdsByPublicKeyHashes;
  let maxIdentitiesPerRequest;
  let rootTreeMock;
  let publicKeyToIdentityIdStoreRootTreeLeafMock;

  beforeEach(function beforeEach() {
    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    maxIdentitiesPerRequest = 5;

    rootTreeMock = {
      getFullProof: this.sinon.stub(),
    };

    publicKeyToIdentityIdStoreRootTreeLeafMock = this.sinon.stub();

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
      maxIdentitiesPerRequest,
      rootTreeMock,
      publicKeyToIdentityIdStoreRootTreeLeafMock,
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

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[0])
      .resolves(identityIds[0]);

    publicKeyIdentityIdRepositoryMock
      .fetch
      .withArgs(publicKeyHashes[1])
      .resolves(identityIds[1]);

    identityIdsByPublicKeyHashes = [
      identityIds[0],
      identityIds[1],
      Buffer.alloc(0),
    ];
  });

  it('should throw an error if maximum requested items exceeded', async () => {
    const params = {};
    const data = { publicKeyHashes };

    maxIdentitiesPerRequest = 1;

    identityIdsByPublicKeyHashesQueryHandler = identityIdsByPublicKeyHashesQueryHandlerFactory(
      publicKeyIdentityIdRepositoryMock,
      maxIdentitiesPerRequest,
      rootTreeMock,
      publicKeyToIdentityIdStoreRootTreeLeafMock,
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
    const params = {};
    const data = { publicKeyHashes };

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, {});

    expect(publicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    const value = await cbor.encodeAsync({
      data: identityIdsByPublicKeyHashes,
    });

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(value);
  });

  it('should return identity id map with proof', async () => {
    const params = {};
    const data = { publicKeyHashes };
    const proof = {
      rootTreeProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
      storeTreeProof: Buffer.from('03046b657931060076616c75653103046b657932060076616c75653210', 'hex'),
    };

    rootTreeMock.getFullProof.returns(proof);

    const result = await identityIdsByPublicKeyHashesQueryHandler(params, data, { prove: 'true' });

    expect(publicKeyIdentityIdRepositoryMock.fetch.callCount).to.equal(
      publicKeyHashes.length,
    );

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(0).args).to.deep.equal([
      publicKeyHashes[0],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(1).args).to.deep.equal([
      publicKeyHashes[1],
    ]);

    expect(publicKeyIdentityIdRepositoryMock.fetch.getCall(2).args).to.deep.equal([
      publicKeyHashes[2],
    ]);

    const value = await cbor.encodeAsync({
      data: identityIdsByPublicKeyHashes,
      proof,
    });

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
    expect(result.value).to.deep.equal(value);
    expect(rootTreeMock.getFullProof).to.be.calledOnce();
    expect(rootTreeMock.getFullProof.getCall(0).args).to.deep.equal([
      publicKeyToIdentityIdStoreRootTreeLeafMock,
      publicKeyHashes,
    ]);
  });
});
