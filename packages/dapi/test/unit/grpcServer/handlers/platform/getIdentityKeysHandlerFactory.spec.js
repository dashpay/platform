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

const {
  GetIdentityKeysResponseV0,
} = GetIdentityKeysResponse;

const {
  Keys,
} = GetIdentityKeysResponseV0;

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityKeysHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityKeysHandlerFactory');

describe('getIdentityKeysHandlerFactory', () => {
  let call;
  let getIdentityKeysHandler;
  let fetchIdentityKeysMock;
  let request;
  let id;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getIdentityId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    fetchIdentityKeysMock = this.sinon.stub();

    getIdentityKeysHandler = getIdentityKeysHandlerFactory({
      fetchIdentityKeys: fetchIdentityKeysMock,
    });
  });

  it('should return identity keys', async () => {
    const keysBytesList = [
      Buffer.from('key1'),
    ];

    const keys = new Keys();
    keys.setKeysBytesList(keysBytesList);

    const response = new GetIdentityKeysResponse()
      .setV0(new GetIdentityKeysResponseV0().setKeys(keys));

    fetchIdentityKeysMock.resolves(response.serializeBinary());

    const result = await getIdentityKeysHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityKeysResponse);

    const keysBytes = result.getV0().getKeys().getKeysBytesList();

    expect(keysBytes).to.deep.equal(keysBytesList);

    expect(fetchIdentityKeysMock).to.be.calledOnceWith(call.request);
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetIdentityKeysResponse()
      .setV0(new GetIdentityKeysResponseV0().setProof(proofMock));

    fetchIdentityKeysMock.resolves(response.serializeBinary());

    const result = await getIdentityKeysHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityKeysResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchIdentityKeysMock).to.be.calledOnceWith(call.request);
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getIdentityId.returns(null);

    try {
      await getIdentityKeysHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('identity id is not specified');
      expect(fetchIdentityKeysMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchIdentityKeysMock.throws(abciResponseError);

    try {
      await getIdentityKeysHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
