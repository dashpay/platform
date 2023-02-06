const calculateStateTransitionFeeFromOperationsFactory = require('../../../lib/stateTransition/fee/calculateStateTransitionFeeFromOperationsFactory');
const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');

describe('calculateStateTransitionFeeFromOperationsFactory', () => {
  let stateTransition;
  let calculateStateTransitionFeeFromOperations;
  let calculateOperationFeesMock;

  beforeEach(async function beforeEach() {
    calculateOperationFeesMock = this.sinonSandbox.stub();

    calculateStateTransitionFeeFromOperations = calculateStateTransitionFeeFromOperationsFactory(
      calculateOperationFeesMock,
    );
  });

  it('should calculate fee based on executed operations', async () => {
    const identityId = generateRandomIdentifier();
    const storageFee = 10000;
    const processingFee = 1000;
    const totalRefunds = 1000 + 500;
    const requiredAmount = storageFee - totalRefunds;
    const desiredAmount = storageFee + processingFee - totalRefunds;
    const feeRefunds = [
      {
        identifier: identityId.toBuffer(),
        creditsPerEpoch: {
          0: 1000,
          1: 500,
        },
      },
    ];

    const calculatedOperationsFeesResult = {
      storageFee,
      processingFee,
      feeRefunds,
    };

    calculateOperationFeesMock.returns(calculatedOperationsFeesResult);

    const result = calculateStateTransitionFeeFromOperations(stateTransition, identityId);

    expect(result).to.deep.equal({
      storageFee,
      processingFee,
      requiredAmount,
      desiredAmount,
      feeRefunds,
      totalRefunds: 1500,
    });
  });
});
