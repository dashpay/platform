const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityKeysResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityKeysHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityKeysHandlerFactory');

describe('getIdentityKeysHandlerFactory', () => {
  let call;
  let getIdentityKeysHandler;
  let driveClientMock;
  let request;
  let id;
  let proofFixture;
  let proofMock;
  let response;
  let proofResponse;
  let keysBytesList;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdentityId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    keysBytesList = [
      Buffer.from('key1'),
    ];

    const keys = new GetIdentityKeysResponse.Keys();
    keys.setKeysBytesList(keysBytesList);

    response = new GetIdentityKeysResponse();
    response.setKeys(keys);

    proofResponse = new GetIdentityKeysResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchIdentityKeys: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityKeysHandler = getIdentityKeysHandlerFactory(driveClientMock);
  });

  it('should return identity keys', async () => {
    const result = await getIdentityKeysHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityKeysResponse);

    const keysBytes = result.getKeys().getKeysBytesList();

    expect(keysBytes).to.deep.equal(keysBytesList);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchIdentityKeys: this.sinon.stub().resolves(proofResponse.serializeBinary()),
    };

    getIdentityKeysHandler = getIdentityKeysHandlerFactory(driveClientMock);

    const result = await getIdentityKeysHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityKeysResponse);

    const keysBytes = result.getKeys();
    expect(keysBytes).to.be.undefined();

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchIdentityKeys).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdentityId.returns(null);

    try {
      await getIdentityKeysHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(driveClientMock.fetchIdentityKeys).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchIdentityKeys.throws(abciResponseError);

    try {
      await getIdentityKeysHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
