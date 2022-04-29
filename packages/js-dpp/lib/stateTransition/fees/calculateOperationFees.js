/**
 * Calculate cpu and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {{ storageCost: number, cpuCost: number }}
 */
function calculateOperationFees(operations) {
  const costs = {
    storageCost: 0.0,
    cpuCost: 0.0,
  };

  operations.forEach((operation) => {
    costs.cpuCost += operation.getCpuCost();
    costs.storageCost += operation.getStorageCost();
  });

  return costs;
}

module.exports = calculateOperationFees;
