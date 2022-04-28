/**
 * Calculate fees based on operations
 * 
 * @param {Operation[]} operations 
 * 
 * @returns {number}
 */
function calculateFees(operations) {
  const costs = {
    storageCost: 0.0,
    cpuCost: 0.0,
  };

  for (const operation of operations) {
    costs.cpuCost += operation.getCpuCost();
    costs.storageCost += operation.getStorageCost();
  }

  // TODO: calculate using formula

  return 0.0;
}

module.exports = calculateFees;
