const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const DashDriveAdapter = require('../../../lib/api/dashDriveAdapter');

const { expect } = chai;
chai.use(chaiAsPromised);

describe('DashDriveAdapter', () => {
  describe('constructor', () => {
    it('Should create dash drive client with given options', () => {
      const dashDrive = new DashDriveAdapter({ host: '127.0.0.1', port: 3000 });

      expect(dashDrive.client.options.host).to.be.equal('127.0.0.1');
      expect(dashDrive.client.options.port).to.be.equal(3000);
    });
  });

  describe('#addSTPacket', () => {
    it('Should call \'addStPacket\' RPC with the given parameters', async () => {
      const dashDrive = new DashDriveAdapter({ host: '127.0.0.1', port: 3000 });

      const packet = 'packet';
      const method = 'addSTPacket';

      const expectedPacketId = 'packetid';

      sinon.stub(dashDrive.client, 'request')
        .withArgs(method, { packet })
        .returns(Promise.resolve({ result: expectedPacketId }));

      expect(dashDrive.client.request.callCount).to.be.equal(0);

      const actualPacketId = await dashDrive.addSTPacket(packet);

      expect(dashDrive.client.request.callCount).to.be.equal(1);
      expect(actualPacketId).to.be.equal(expectedPacketId);
    });
  });

  describe('#fetchDapContract', () => {
    it('Should call \'fetchDapContract\' RPC with the given parameters', async () => {
      const dashDrive = new DashDriveAdapter({ host: '127.0.0.1', port: 3000 });

      const dapId = 'dapid';
      const method = 'fetchDapContract';

      const expectedDapContract = { dapId };

      sinon.stub(dashDrive.client, 'request')
        .withArgs(method, { dapId })
        .returns(Promise.resolve({ result: expectedDapContract }));

      expect(dashDrive.client.request.callCount).to.be.equal(0);

      const actualDapContract = await dashDrive.fetchDapContract(dapId);

      expect(dashDrive.client.request.callCount).to.be.equal(1);
      expect(actualDapContract).to.be.equal(expectedDapContract);
      expect(actualDapContract).not.to.be.equal({ dapId: 'randomid' });
    });
  });

  describe('#fetchDapObjects', () => {
    it('Should call \'fetchDapObjects\' RPC with the given parameters', async () => {
      const dashDrive = new DashDriveAdapter({ host: '127.0.0.1', port: 3000 });

      const dapId = 'dapid';
      const type = 'contact';
      const options = { where: { id: 1 } };
      const method = 'fetchDapObjects';

      const expectedDapObjects = [{ dapId, id: 1 }];


      sinon.stub(dashDrive.client, 'request')
        .withArgs(method, { dapId, type, options })
        .returns(Promise.resolve({ result: expectedDapObjects }));

      expect(dashDrive.client.request.callCount).to.be.equal(0);

      const actualDapObjects = await dashDrive.fetchDapObjects(dapId, type, options);

      expect(dashDrive.client.request.callCount).to.be.equal(1);
      expect(actualDapObjects).to.be.equal(expectedDapObjects);
      expect(actualDapObjects).not.to.be.equal([{ dapId, id: 2 }]);
    });
  });
});
