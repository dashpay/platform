const chai = require('chai');
const sinon = require('sinon');
const cbor = require('cbor');

const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');

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
    GetIdentityRequest,
    GetIdentityResponse,
    GetProofsRequest,
    GetProofsResponse,
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

      const request = new GetDataContractRequest();
      request.setId(contractId);
      request.setProve(false);

      const response = new GetDataContractResponse();
      response.setDataContract(data);
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

      const request = new GetDocumentsRequest();
      request.setDataContractId(contractId);
      request.setDocumentType(type);
      request.setWhere(cbor.encode({ where: options.where }));

      const response = new GetDocumentsResponse();
      const documents = new GetDocumentsResponse.Documents();
      documents.setDocumentsList([]);
      response.setDocuments(documents);
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

      const request = new GetIdentityRequest();
      request.setId(identityId);

      const response = new GetIdentityResponse();
      response.setIdentity(data);
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

      const request = new GetIdentitiesByPublicKeyHashesRequest();
      request.setPublicKeyHashesList(publicKeyHashes);
      request.setProve(false);

      const response = new GetIdentitiesByPublicKeyHashesResponse();
      const identitiesList = new GetIdentitiesByPublicKeyHashesResponse.Identities();
      identitiesList.setIdentitiesList([identity.toBuffer()]);
      response.setIdentities(identitiesList);
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
      request.setIdentitiesList(identityIds.map((id) => {
        const { IdentityRequest } = GetProofsRequest;
        const identityRequest = new IdentityRequest();
        identityRequest.setIdentityId(id);
        identityRequest.setRequestType(IdentityRequest.Type.FULL_IDENTITY);
        return identityRequest;
      }));

      const response = new GetProofsResponse();
      response.setProof(new Proof());
      response.setMetadata(new ResponseMetadata());
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
});
