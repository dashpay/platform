const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');

describe('DapContract', () => {
  it('should serialize DapContract', () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const reference = new Reference();
    const schema = {};
    const version = 2;
    const isDeleted = false;
    const previousVersions = [];
    const dapContract = new DapContract(
      dapId,
      dapName,
      reference,
      schema,
      version,
      isDeleted,
      previousVersions,
    );

    const dapContractSerialized = dapContract.toJSON();
    expect(dapContractSerialized).to.deep.equal({
      dapId,
      dapName,
      reference,
      schema,
      version,
      isDeleted,
      previousVersions,
    });
  });

  it('should add revision to DapContract', () => {
    const firstDapId = '1234';
    const firstDapName = 'DashPay';
    const firstReference = new Reference();
    const firstSchema = {};
    const firstVersion = 1;
    const firstVersionDeleted = false;
    const firstPreviousVersions = [];
    const firstDapContract = new DapContract(
      firstDapId,
      firstDapName,
      firstReference,
      firstSchema,
      firstVersion,
      firstVersionDeleted,
      firstPreviousVersions,
    );

    const secondDapId = '5678';
    const secondDapName = 'DashPay';
    const secondReference = new Reference();
    const secondSchema = {};
    const secondVersion = 2;
    const secondVersionDeleted = false;
    const secondPreviousVersions = [firstDapContract.currentRevision()];
    const secondDapContract = new DapContract(
      secondDapId,
      secondDapName,
      secondReference,
      secondSchema,
      secondVersion,
      secondVersionDeleted,
      secondPreviousVersions,
    );

    const thirdDapId = '9999';
    const thirdDapName = 'DashPay';
    const thirdReference = new Reference();
    const thirdSchema = {};
    const thirdVersion = 2;
    const thirdVersionDeleted = false;
    const thirdPreviousVersions = [];
    const thirdDapContract = new DapContract(
      thirdDapId,
      thirdDapName,
      thirdReference,
      thirdSchema,
      thirdVersion,
      thirdVersionDeleted,
      thirdPreviousVersions,
    );

    thirdDapContract.addRevision(secondDapContract);

    expect(thirdDapContract.getPreviousVersions()).to.be.deep.equal([
      firstDapContract.currentRevision(),
      secondDapContract.currentRevision(),
    ]);
  });
});
