/**
 * Calculate fees based on operations
 * 
 * @param {Operation[]} operations 
 * 
 * @returns {Object}
 */
function calculateFees(operations) {
  const fees = {
    storageCost: 0.0,
    cpuCost: 0.0,
  };

  for (const operation of operations) {
    fees.cpuCost += operation.getCpuCost();
    fees.storageCost += operation.getStorageCost();
  }

  return fees;
}

module.exports = calculateFees;
