const chai = require('chai');
const sinon = require('sinon');
const cbor = require('cbor');

const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');

const { BytesValue, UInt32Value } = require('google-protobuf/google/protobuf/wrappers_pb');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const {
  v0: {
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    GetDataContractRequest,
    GetDataContractResponse,
    GetDocumentsRequest,
    GetDocumentsResponse,
    GetEpochsInfoRequest,
    GetEpochsInfoResponse,
    GetIdentityRequest,
    GetIdentityResponse,
    GetProofsRequest,
    GetProofsResponse,
    GetVersionUpgradeStateRequest,
    GetVersionUpgradeStateResponse,
    GetVersionUpgradeVoteStatusRequest,
    GetVersionUpgradeVoteStatusResponse,
    Proof,
    ResponseMetadata,
  },
} = require('@dashevo/dapi-grpc');

const DriveClient = require('../../../../lib/externalApis/drive/DriveClient');

const RPCError = require('../../../../lib/rpcServer/RPCError');

chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;

describe('DriveClient', () => {
  describe('constructor', () => {
    it('Should create drive client with given options', () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      expect(drive.client.options.host).to.be.equal('127.0.0.1');
      expect(drive.client.options.port).to.be.equal(3000);
    });
  });

  it('should throw RPCError if JSON RPC call failed', async () => {
    const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

    const error = new Error('Some RPC error');

    sinon.stub(drive.client, 'request')
      .resolves({ error });

    try {
      await drive.fetchDataContract(new GetDataContractRequest());
    } catch (e) {
      expect(e).to.be.an.instanceOf(RPCError);
      expect(e.message).to.be.equal(error.message);
      expect(e.code).to.be.equal(-32602);
    }
  });

  it('should throw ABCI error if response have one', async () => {
    const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

    sinon.stub(drive.client, 'request')
      .resolves({
        result: {
          response: {
            code: GrpcErrorCodes.INVALID_ARGUMENT,
            info: cbor.encode({
              data: {
                name: 'someData',
              },
              message: 'some message',
            }).toString('base64'),
          },
        },
      });

    try {
      await drive.fetchDataContract(new GetDataContractRequest());
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getCode()).to.equal(3);
      expect(e.getMessage()).to.equal('some message');
      expect(e.getRawMetadata()).to.deep.equal({
        'drive-error-data-bin': cbor.encode({
          name: 'someData',
        }),
      });
    }
  });

  describe('#fetchDataContract', () => {
    it('Should call \'fetchContract\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const contractId = 'someId';
      const data = Buffer.from('someData');

      const { GetDataContractRequestV0 } = GetDataContractRequest;
      const request = new GetDataContractRequest();
      request.setV0(
        new GetDataContractRequestV0()
          .setId(contractId)
          .setProve(false),
      );

      const { GetDataContractResponseV0 } = GetDataContractResponse;
      const response = new GetDataContractResponse();
      response.setV0(
        new GetDataContractResponseV0()
          .setDataContract(data),
      );
      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchDataContract(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/dataContract',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });
      expect(result).to.be.deep.equal(responseBytes);
    });
  });

  describe('#fetchDocuments', () => {
    it('Should call \'fetchDocuments\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const contractId = 'someId';
      const type = 'object';
      const options = {
        where: 'id === someId',
      };

      const { GetDocumentsRequestV0 } = GetDocumentsRequest;
      const request = new GetDocumentsRequest();
      request.setV0(
        new GetDocumentsRequestV0()
          .setDataContractId(contractId)
          .setDocumentType(type)
          .setWhere(cbor.encode({ where: options.where })),
      );

      const { GetDocumentsResponseV0 } = GetDocumentsResponse;
      const response = new GetDocumentsResponse();
      response.setV0(
        new GetDocumentsResponseV0()
          .setDocuments(new GetDocumentsResponseV0.Documents().setDocumentsList([])),
      );

      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchDocuments(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/dataContract/documents',
        data: Buffer.from(request.serializeBinary()).toString('hex'), // cbor encoded empty object
      });
      expect(result).to.be.deep.equal(responseBytes);
    });
  });

  describe('#fetchIdentity', () => {
    it('Should call \'fetchIdentity\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const identityId = 'someId';
      const data = Buffer.from('someData');

      const { GetIdentityRequestV0 } = GetIdentityRequest;
      const request = new GetIdentityRequest();
      request.setV0(
        new GetIdentityRequestV0()
          .setId(identityId),
      );

      const { GetIdentityResponseV0 } = GetIdentityResponse;
      const response = new GetIdentityResponse();
      response.setV0(
        new GetIdentityResponseV0()
          .setIdentity(data),
      );
      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchIdentity(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/identity',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });
      expect(result).to.be.deep.equal(responseBytes);
    });
  });

  describe('#fetchIdentitiesByPublicKeyHashes', () => {
    it('Should call \'fetchIdentitiesByPublicKeyHashes\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const identity = await getIdentityFixture();

      const publicKeyHashes = [Buffer.alloc(1)];

      const { GetIdentitiesByPublicKeyHashesRequestV0 } = GetIdentitiesByPublicKeyHashesRequest;
      const request = new GetIdentitiesByPublicKeyHashesRequest();
      request.setV0(
        new GetIdentitiesByPublicKeyHashesRequestV0()
          .setPublicKeyHashesList(publicKeyHashes)
          .setProve(false),
      );

      const {
        IdentitiesByPublicKeyHashes,
        PublicKeyHashIdentityEntry,
        GetIdentitiesByPublicKeyHashesResponseV0,
      } = GetIdentitiesByPublicKeyHashesResponse;

      const response = new GetIdentitiesByPublicKeyHashesResponse();
      response.setV0(
        new GetIdentitiesByPublicKeyHashesResponseV0().setIdentities(
          new IdentitiesByPublicKeyHashes()
            .setIdentityEntriesList([
              new PublicKeyHashIdentityEntry()
                .setPublicKeyHash(publicKeyHashes[0])
                .setValue(new BytesValue().setValue(identity.toBuffer())),
            ]),
        ),
      );
      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchIdentitiesByPublicKeyHashes(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/identities/by-public-key-hash',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });
      expect(result).to.be.deep.equal(responseBytes);
    });
  });

  describe('#fetchProofs', () => {
    it('should call \'fetchProofs\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const identityIds = [Buffer.from('id')];

      const request = new GetProofsRequest();
      const { GetProofsRequestV0 } = GetProofsRequest;
      request.setV0(
        new GetProofsRequestV0()
          .setIdentitiesList(identityIds.map((id) => {
            const { IdentityRequest } = GetProofsRequestV0;
            const identityRequest = new IdentityRequest();
            identityRequest.setIdentityId(id);
            identityRequest.setRequestType(IdentityRequest.Type.FULL_IDENTITY);
            return identityRequest;
          })),
      );

      const { GetProofsResponseV0 } = GetProofsResponse;
      const response = new GetProofsResponse();
      response.setV0(
        new GetProofsResponseV0()
          .setProof(new Proof())
          .setMetadata(new ResponseMetadata()),
      );

      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchProofs(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/proofs',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });

      expect(result).to.be.deep.equal(
        responseBytes,
      );
    });
  });

  describe('#fetchEpochsInfo', () => {
    it('should call \'fetchEpochsInfo\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const { GetEpochsInfoRequestV0 } = GetEpochsInfoRequest;
      const request = new GetEpochsInfoRequest();
      request.setV0(
        new GetEpochsInfoRequestV0()
          .setStartEpoch(new UInt32Value([1]))
          .setCount(1),
      );

      const { GetEpochsInfoResponseV0 } = GetEpochsInfoResponse;
      const response = new GetEpochsInfoResponse();
      const { EpochInfo, EpochInfos } = GetEpochsInfoResponseV0;
      response.setV0(
        new GetEpochsInfoResponseV0()
          .setEpochs(new EpochInfos()
            .setEpochInfosList([new EpochInfo()
              .setNumber(1)
              .setFirstBlockHeight(1)
              .setFirstCoreBlockHeight(1)
              .setStartTime(Date.now())
              .setFeeMultiplier(1.1)])),
      );

      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchEpochsInfo(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/epochInfos',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });

      expect(result).to.be.deep.equal(
        responseBytes,
      );
    });
  });

  describe('#fetchVersionUpgradeVoteStatus', () => {
    it('should call \'fetchEpochsInfo\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const { GetVersionUpgradeVoteStatusRequestV0 } = GetVersionUpgradeVoteStatusRequest;
      const request = new GetVersionUpgradeVoteStatusRequest();
      request.setV0(
        new GetVersionUpgradeVoteStatusRequestV0()
          .setStartProTxHash(Buffer.alloc(32))
          .setCount(1),
      );

      const { GetVersionUpgradeVoteStatusResponseV0 } = GetVersionUpgradeVoteStatusResponse;
      const response = new GetVersionUpgradeVoteStatusResponse();
      const { VersionSignal, VersionSignals } = GetVersionUpgradeVoteStatusResponseV0;
      response.setV0(
        new GetVersionUpgradeVoteStatusResponseV0()
          .setVersions(
            new VersionSignals()
              .setVersionSignalsList([
                new VersionSignal()
                  .setProTxHash(Buffer.alloc(32))
                  .setVersion(10),
              ]),

          ),
      );

      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchVersionUpgradeVoteStatus(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/versionUpgrade/voteStatus',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });

      expect(result).to.be.deep.equal(
        responseBytes,
      );
    });
  });

  describe('#fetchVersionUpgradeState', () => {
    it('should call \'fetchEpochsInfo\' RPC with the given parameters', async () => {
      const drive = new DriveClient({ host: '127.0.0.1', port: 3000 });

      const { GetVersionUpgradeStateRequestV0 } = GetVersionUpgradeStateRequest;
      const request = new GetVersionUpgradeStateRequest();
      request.setV0(new GetVersionUpgradeStateRequestV0());

      const { GetVersionUpgradeStateResponseV0 } = GetVersionUpgradeStateResponse;
      const response = new GetVersionUpgradeStateResponse();
      const { Versions, VersionEntry } = GetVersionUpgradeStateResponseV0;
      response.setV0(
        new GetVersionUpgradeStateResponseV0()
          .setVersions(
            new Versions()
              .setVersionsList([
                new VersionEntry()
                  .setVersionNumber(1)
                  .setVoteCount(10),
              ]),
          ),
      );

      const responseBytes = response.serializeBinary();

      sinon.stub(drive.client, 'request')
        .resolves({
          result: {
            response: { code: 0, value: responseBytes },
          },
        });

      const result = await drive.fetchVersionUpgradeState(request);

      expect(drive.client.request).to.have.been.calledOnceWithExactly('abci_query', {
        path: '/versionUpgrade/state',
        data: Buffer.from(request.serializeBinary()).toString('hex'),
      });

      expect(result).to.be.deep.equal(
        responseBytes,
      );
    });
  });
});
