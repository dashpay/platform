/**
 * Calculate processing and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {{ storageCost: number, processingCost: number }}
 */
function calculateOperationFees(operations) {
  const costs = {
    storageCost: 0.0,
    processingCost: 0.0,
  };

  operations.forEach((operation) => {
    costs.processingCost += operation.getProcessingCost();
    costs.storageCost += operation.getStorageCost();
  });

  return costs;
}

module.exports = calculateOperationFees;
