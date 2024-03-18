import calculatePaymentQueuePosition from '../../../src/core/calculatePaymentQueuePosition.js';

describe('calculatePaymentQueuePosition', () => {
  it('should just work', async () => {
    const mockDmnState = { lastPaidHeight: 1, PoSeRevivedHeight: 0, registeredHeight: 1 };

    const position = calculatePaymentQueuePosition(mockDmnState, 0, 3, 10);

    expect(position).to.equal(3);
  });
});
