const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityByPublicKeyHashResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const {
  GetIdentityByPublicKeyHashResponseV0,
} = GetIdentityByPublicKeyHashResponse;

/* eslint-disable import/no-extraneous-dependencies */
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityByPublicKeyHashHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityByPublicKeyHashHandlerFactory');

describe('getIdentityByPublicKeyHashHandlerFactory', () => {
  let call;
  let getIdentityByPublicKeyHashHandler;
  let request;
  let fetchIdentityByPublicKeyHashMock;

  beforeEach(async function beforeEach() {
    request = {
      getPublicKeyHash: this.sinon.stub().returns(
        Buffer.alloc(1, 1),
      ),
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    fetchIdentityByPublicKeyHashMock = this.sinon.stub();

    getIdentityByPublicKeyHashHandler = getIdentityByPublicKeyHashHandlerFactory({
      fetchIdentityByPublicKeyHash: fetchIdentityByPublicKeyHashMock,
    });
  });

  it('should return identity', async () => {
    const identityFixture = await getIdentityFixture();

    const response = new GetIdentityByPublicKeyHashResponse()
      .setV0(new GetIdentityByPublicKeyHashResponseV0().setIdentity(identityFixture.toBuffer()));

    fetchIdentityByPublicKeyHashMock.resolves(response.serializeBinary());

    const result = await getIdentityByPublicKeyHashHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityByPublicKeyHashResponse);

    const identity = result.getV0().getIdentity();

    expect(identity).to.deep.equal(identityFixture.toBuffer());

    expect(fetchIdentityByPublicKeyHashMock).to.be.calledOnceWith(
      call.request,
    );
  });

  it('should return proof', async () => {
    const proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    const proofMock = new Proof();

    proofMock.setGrovedbProof(proofFixture.merkleProof);

    const response = new GetIdentityByPublicKeyHashResponse().setV0(
      new GetIdentityByPublicKeyHashResponseV0().setProof(proofMock),
    );

    fetchIdentityByPublicKeyHashMock.resolves(response.serializeBinary());

    const result = await getIdentityByPublicKeyHashHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityByPublicKeyHashResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(fetchIdentityByPublicKeyHashMock).to.be.calledOnceWith(
      call.request,
    );
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getPublicKeyHash.returns([]);

    try {
      await getIdentityByPublicKeyHashHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('No public key hash is provided');
      expect(fetchIdentityByPublicKeyHashMock).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    fetchIdentityByPublicKeyHashMock.throws(abciResponseError);

    try {
      await getIdentityByPublicKeyHashHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
