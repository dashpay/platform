const chai = require('chai');
const sinon = require('sinon');

const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');

const DriveAdapter = require('../../../../lib/externalApis/driveAdapter');

chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;

describe('DriveAdapter', () => {
  describe('constructor', () => {
    it('Should create drive client with given options', () => {
      const drive = new DriveAdapter({ host: '127.0.0.1', port: 3000 });

      expect(drive.client.options.host).to.be.equal('127.0.0.1');
      expect(drive.client.options.port).to.be.equal(3000);
    });
  });

  describe('#addSTPacket', () => {
    it('Should call \'addStPacket\' RPC with the given parameters', async () => {
      const drive = new DriveAdapter({ host: '127.0.0.1', port: 3000 });

      const rawSTPacket = 'stPacket';
      const rawStateTransition = 'stateTransition';
      const method = 'addSTPacket';

      sinon.stub(drive.client, 'request')
        .withArgs(method, { stateTransition: rawStateTransition, stPacket: rawSTPacket })
        .returns(Promise.resolve({ result: undefined }));

      expect(drive.client.request.callCount).to.be.equal(0);

      const result = await drive.addSTPacket(rawStateTransition, rawSTPacket);

      expect(drive.client.request.callCount).to.be.equal(1);
      expect(result).to.be.undefined();
    });
  });

  describe('#fetchContract', () => {
    it('Should call \'fetchContract\' RPC with the given parameters', async () => {
      const drive = new DriveAdapter({ host: '127.0.0.1', port: 3000 });

      const contractId = 'contractId';
      const method = 'fetchContract';

      const expectedContract = { contractId };

      sinon.stub(drive.client, 'request')
        .withArgs(method, { contractId })
        .returns(Promise.resolve({ result: expectedContract }));

      expect(drive.client.request.callCount).to.be.equal(0);

      const actualContract = await drive.fetchContract(contractId);

      expect(drive.client.request.callCount).to.be.equal(1);
      expect(actualContract).to.be.equal(expectedContract);
      expect(actualContract).not.to.be.equal({ contractId: 'randomid' });
    });
  });

  describe('#fetchDocuments', () => {
    it('Should call \'fetchDocuments\' RPC with the given parameters', async () => {
      const drive = new DriveAdapter({ host: '127.0.0.1', port: 3000 });

      const contractId = 'contractId';
      const type = 'contact';
      const options = { where: { id: 1 } };
      const method = 'fetchDocuments';

      const expectedDocuments = [{ contractId, id: 1 }];


      sinon.stub(drive.client, 'request')
        .withArgs(method, { contractId, type, options })
        .returns(Promise.resolve({ result: expectedDocuments }));

      expect(drive.client.request.callCount).to.be.equal(0);

      const actualDocuments = await drive.fetchDocuments(contractId, type, options);

      expect(drive.client.request.callCount).to.be.equal(1);
      expect(actualDocuments).to.be.equal(expectedDocuments);
      expect(actualDocuments).not.to.be.equal([{ contractId, id: 2 }]);
    });
  });
});
