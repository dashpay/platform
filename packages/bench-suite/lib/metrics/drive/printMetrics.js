const { Table } = require('console-table-printer');

const mathjs = require('mathjs');

/**
 * @param {string} title
 * @param {Object[]} metrics
 * @param {Object} config
 */
function printMetrics(title, metrics, config) {
  // eslint-disable-next-line no-console
  console.log(`${metrics.length} "${title}" metrics collected:`);

  const overall = [];
  const validateBasic = [];
  const validateFee = [];
  const validateSignature = [];
  const validateState = [];
  const apply = [];

  metrics.forEach((metric) => {
    overall.push(metric.timings.overall);
    validateBasic.push(metric.timings.validateBasic);
    validateFee.push(metric.timings.validateFee);
    validateSignature.push(metric.timings.validateSignature);
    validateState.push(metric.timings.validateState);
    apply.push(metric.timings.apply);
  });

  const timingTable = new Table({
    columns: [
      { name: 'overall' },
      { name: 'validateBasic' },
      { name: 'validateFee' },
      { name: 'validateSignature' },
      { name: 'validateState' },
      { name: 'apply' },
    ],
  });

  if (config.avgOnly) {
    timingTable.addRow({
      overall: '...',
      validateBasic: '...',
      validateFee: '...',
      validateSignature: '...',
      validateState: '...',
      apply: '...',
    });
  } else {
    timingTable.addRows(
      metrics.map((metric) => metric.timings),
    );
  }

  const avgFunction = mathjs[config.avgFunction];

  timingTable.addRow({
    overall: avgFunction(overall)
      .toFixed(3),
    validateBasic: avgFunction(validateBasic)
      .toFixed(3),
    validateFee: avgFunction(validateFee)
      .toFixed(3),
    validateSignature: avgFunction(validateSignature)
      .toFixed(3),
    validateState: avgFunction(validateState)
      .toFixed(3),
    apply: avgFunction(apply)
      .toFixed(3),
  }, {
    color: 'white_bold',
    separator: true,
  });

  timingTable.printTable();

  // eslint-disable-next-line no-console
  console.log(`\n\n"${title}" fees:`);

  const feeTable = new Table({
    columns: [
      { name: 'predicted storage' },
      { name: 'actual storage' },
      { name: 'predicted processing' },
      { name: 'actual processing' },
      { name: 'predicted final' },
      { name: 'actual final' },
      { name: 'predicted operations' },
      { name: 'actual operations' },
    ],
  });

  const {
    predicted,
    actual,
  } = metrics[0].fees;

  feeTable.addRow({
    'predicted storage': predicted.storage,
    'actual storage': actual.storage,
    'predicted processing': predicted.processing,
    'actual processing': actual.processing,
    'predicted final': predicted.final,
    'actual final': actual.final,
    'predicted operations': predicted.operations.length,
    'actual operations': actual.operations.length,
  });

  feeTable.printTable();

  // eslint-disable-next-line no-console
  console.log(`\n\n${predicted.operations.length} "${title}" predicted fee operations:\n`);

  predicted.operations.forEach((operation) => {
    // eslint-disable-next-line no-console
    console.log(operation);
  });

  // eslint-disable-next-line no-console
  console.log(`\n\n${actual.operations.length} "${title}" actual fee operations:\n`);

  actual.operations.forEach((operation) => {
    // eslint-disable-next-line no-console
    console.log(operation);
  });
}

module.exports = printMetrics;
