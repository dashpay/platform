const calculateStateTransitionFeeFactory = require('../../../lib/stateTransition/fee/calculateStateTransitionFeeFactory');

const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

describe('calculateStateTransitionFeeFactory', () => {
  let stateTransition;
  let calculateStateTransitionFee;
  let stateRepositoryMock;
  let calculateOperationFeesMock;

  beforeEach(async function beforeEach() {
    const privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';

    stateTransition = getIdentityCreateTransitionFixture();
    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    calculateOperationFeesMock = this.sinonSandbox.stub();

    calculateStateTransitionFee = calculateStateTransitionFeeFactory(
      stateRepositoryMock,
      calculateOperationFeesMock,
    );
  });

  it('should calculate fee based on executed operations', async () => {
    const storageFee = 10000;
    const processingFee = 1000;
    const totalRefunds = 1000 + 500;
    const requiredAmount = storageFee - totalRefunds;
    const desiredAmount = storageFee + processingFee - totalRefunds;

    const calculatedOperationsFeesResult = {
      storageFee,
      processingFee,
      feeRefunds: [
        {
          identifier: stateTransition.getOwnerId().toBuffer(),
          creditsPerEpoch: {
            0: 1000,
            1: 500,
          },
        },
      ],
    };

    calculateOperationFeesMock.returns(calculatedOperationsFeesResult);

    const result = await calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(desiredAmount);

    const lastCalculatedFeeDetails = stateTransition.getExecutionContext()
      .getLastCalculatedFeeDetails();

    expect(lastCalculatedFeeDetails).to.be.deep.equal({
      ...calculatedOperationsFeesResult,
      totalRefunds,
      requiredAmount,
      desiredAmount,
    });
  });
});
