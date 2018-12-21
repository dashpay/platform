const Reference = require('../../../../lib/stateView/Reference');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');

const generateDapObjectId = require('../../../../lib/stateView/dapObject/generateDapObjectId');

describe('DapObject', () => {
  it('should get DapObject ID', async () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const isDeleted = false;
    const slotNumber = 0;
    const dapObjectData = {
      objtype: 'user',
      idx: slotNumber,
      rev: 1,
      act: 0,
    };
    const reference = new Reference();
    const dapObject = new DapObject(blockchainUserId, dapObjectData, reference, isDeleted);
    const dapObjectId = generateDapObjectId(blockchainUserId, slotNumber);
    expect(dapObject.getId()).to.be.equal(dapObjectId);
  });

  it('should serialize DapObject', () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const isDeleted = false;
    const userBio = 'some info here';
    const dapObjectData = {
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 1,
      bio: userBio,
    };
    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
    );
    const previousRevisions = [];
    const dapObject = new DapObject(
      blockchainUserId,
      dapObjectData,
      reference,
      isDeleted,
      previousRevisions,
    );

    const dapObjectSerialized = dapObject.toJSON();
    expect(dapObjectSerialized).to.deep.equal({
      blockchainUserId,
      isDeleted,
      type: dapObjectData.objtype,
      protocolVersion: dapObjectData.pver,
      idx: dapObjectData.idx,
      action: dapObjectData.act,
      revision: dapObjectData.rev,
      data: {
        bio: userBio,
      },
      reference: reference.toJSON(),
      previousRevisions,
    });
  });

  it('should have DapObject action constants', async () => {
    expect(DapObject.ACTION_CREATE).to.be.equal(0);
    expect(DapObject.ACTION_UPDATE).to.be.equal(1);
    expect(DapObject.ACTION_DELETE).to.be.equal(2);
  });

  it('should get DapObject action', async () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const isDeleted = false;
    const dapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    const reference = new Reference();
    const dapObject = new DapObject(blockchainUserId, dapObjectData, reference, isDeleted);
    expect(dapObject.getAction()).to.be.equal(0);
  });

  it('should add revision to DapObject', () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const isDeleted = false;

    const firstDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    const firstReference = new Reference();
    const firstPreviousRevisions = [];
    const firstDapObject = new DapObject(
      blockchainUserId,
      firstDapObjectData,
      firstReference,
      isDeleted,
      firstPreviousRevisions,
    );

    const secondDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 2,
      act: 0,
    };
    const secondReference = new Reference();
    const secondPreviousVersions = [firstDapObject.currentRevision()];
    const secondDapObject = new DapObject(
      blockchainUserId,
      secondDapObjectData,
      secondReference,
      isDeleted,
      secondPreviousVersions,
    );

    const thirdDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 3,
      act: 0,
    };
    const thirdReference = new Reference();
    const thirdPreviousVersions = [];
    const thirdDapObject = new DapObject(
      blockchainUserId,
      thirdDapObjectData,
      thirdReference,
      isDeleted,
      thirdPreviousVersions,
    );
    thirdDapObject.addRevision(secondDapObject);

    expect(thirdDapObject.getPreviousRevisions()).to.be.deep.equal([
      firstDapObject.currentRevision(),
      secondDapObject.currentRevision(),
    ]);
  });

  it('should remove specified revisions of DapObject', () => {
    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const isDeleted = false;

    const firstDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };
    const firstReference = new Reference();
    const firstPreviousRevisions = [];
    const firstDapObject = new DapObject(
      blockchainUserId,
      firstDapObjectData,
      firstReference,
      isDeleted,
      firstPreviousRevisions,
    );

    const secondDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 2,
      act: 0,
    };
    const secondReference = new Reference();
    const secondDapObject = new DapObject(
      blockchainUserId,
      secondDapObjectData,
      secondReference,
      isDeleted,
      [],
    );

    const thirdDapObjectData = {
      id: '1234',
      objtype: 'user',
      idx: 0,
      rev: 3,
      act: 0,
    };
    const thirdReference = new Reference();
    const thirdPreviousVersions = [
      firstDapObject.currentRevision(),
      secondDapObject.currentRevision(),
    ];
    const thirdDapObject = new DapObject(
      blockchainUserId,
      thirdDapObjectData,
      thirdReference,
      isDeleted,
      thirdPreviousVersions,
    );

    secondDapObject.addRevision(thirdDapObject);
    secondDapObject.removeAheadRevisions();

    expect(secondDapObject.getPreviousRevisions()).to.be.deep.equal([
      firstDapObject.currentRevision(),
    ]);
  });

  it('should return original data by calling getOriginalData', () => {
    const originalData = {
      objtype: 'user',
      pver: 1,
      idx: 0,
      rev: 1,
      act: 0,
    };

    const dapObject = new DapObject(
      null,
      originalData,
      null,
      false,
    );

    expect(dapObject.getOriginalData()).to.be.deep.equal(originalData);
  });
});
