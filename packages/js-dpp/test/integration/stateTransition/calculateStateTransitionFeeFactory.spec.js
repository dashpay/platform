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

  it('should throw an error if more than two identities have refunds', async () => {
    const calculatedOperationsFeesResult = {
      storageFee: 10000,
      processingFee: 1000,
      feeRefunds: [
        {
          identifier: stateTransition.getOwnerId().toBuffer(),
          creditsPerEpoch: {
            0: -100,
            1: -50,
          },
        },
        {
          identifier: stateTransition.getOwnerId().toBuffer(),
          creditsPerEpoch: {
            1: -50,
          },
        },
      ],
    };

    calculateOperationFeesMock.returns(calculatedOperationsFeesResult);

    try {
      await calculateStateTransitionFee(stateTransition);

      expect.fail('should fail');
    } catch (e) {
      expect(e.message).to.equals('State Transition removed bytes from several identities that is not defined by protocol');
    }
  });

  it('should throw an error if refunded identity is not owner of state transition', async () => {
    const calculatedOperationsFeesResult = {
      storageFee: 10000,
      processingFee: 1000,
      feeRefunds: [
        {
          identifier: Buffer.alloc(32, 1),
          creditsPerEpoch: {
            0: -100,
            1: -50,
          },
        },
      ],
    };

    calculateOperationFeesMock.returns(calculatedOperationsFeesResult);

    try {
      await calculateStateTransitionFee(stateTransition);

      expect.fail('should fail');
    } catch (e) {
      expect(e.message).to.equals('State Transition removed bytes from different identity');
    }
  });

  it('should calculate fee based on executed operations', async () => {
    const storageFee = 10000;
    const processingFee = 1000;
    const feeRefundsSum = 450 + 995 + 400 + 400;
    const total = storageFee + processingFee - feeRefundsSum;

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

    stateRepositoryMock.calculateStorageFeeDistributionAmountAndLeftovers
      .onCall(0).resolves([995, 400]);

    stateRepositoryMock.calculateStorageFeeDistributionAmountAndLeftovers
      .onCall(1).resolves([450, 400]);

    const result = await calculateStateTransitionFee(stateTransition);

    expect(result).to.equal(total);

    expect(stateRepositoryMock.calculateStorageFeeDistributionAmountAndLeftovers)
      .to.have.been.calledWithExactly(1000, 0);

    expect(stateRepositoryMock.calculateStorageFeeDistributionAmountAndLeftovers)
      .to.have.been.calledWithExactly(500, 1);

    const lastCalculatedFeeDetails = stateTransition.getExecutionContext()
      .getLastCalculatedFeeDetails();

    expect(lastCalculatedFeeDetails).to.be.deep.equal({
      ...calculatedOperationsFeesResult,
      feeRefundsSum,
      total,
    });
  });
});
