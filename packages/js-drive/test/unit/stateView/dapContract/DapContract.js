const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');

describe('DapContract', () => {
  it('should serialize DapContract', () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const reference = new Reference();
    const version = 2;
    const data = {
      dapname: dapName,
      dapver: version,
      schema: {},
    };
    const isDeleted = false;
    const previousVersions = [];
    const dapContract = new DapContract(
      dapId,
      data,
      reference,
      isDeleted,
      previousVersions,
    );

    const dapContractSerialized = dapContract.toJSON();
    expect(dapContractSerialized).to.deep.equal({
      dapId,
      dapName,
      reference,
      data: {
        schema: data.schema,
      },
      version,
      isDeleted,
      previousVersions,
    });
  });

  it('should add revision to DapContract', () => {
    const firstReference = new Reference();
    const firstData = {
      dapver: 1,
    };
    const firstDapContract = new DapContract(
      null,
      firstData,
      firstReference,
      false,
    );

    const secondReference = new Reference();
    const secondData = {
      dapver: 2,
    };
    const secondDapContract = new DapContract(
      null,
      secondData,
      secondReference,
      false,
    );

    secondDapContract.addRevision(firstDapContract);

    const thirdReference = new Reference();
    const thirdData = {
      dapver: 3,
    };
    const thirdDapContract = new DapContract(
      null,
      thirdData,
      thirdReference,
      false,
    );

    thirdDapContract.addRevision(secondDapContract);

    expect(thirdDapContract.getPreviousVersions()).to.be.deep.equal([
      firstDapContract.currentRevision(),
      secondDapContract.currentRevision(),
    ]);
  });

  it('should be able to check `deleted` flag by calling isDeleted');
});
