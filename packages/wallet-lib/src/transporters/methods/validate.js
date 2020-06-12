const logger = require('../../logger');

module.exports = function validate(transporter, silent = false) {
  const { BaseTransporter } = this;
  let isValid = true;
  const expectedKeys = [
    'getAddressSummary',
    'getTransaction',
    'getUTXO',
    'sendTransaction',
    'getIdentityIdByFirstPublicKey',
  ];

  expectedKeys.forEach((key) => {
    if (!transporter[key] && !(transporter.client && transporter.client[key])) {
      isValid = false;
      if (!silent) {
        logger.error(`Invalid Transporter. Expected key :${key}`);
      }
    }
    // BaseTransporter throw only errors (as a template), so if similar it's not implemented
    // as we requires it, we warn and invalid the transporter
    if (transporter[key] === BaseTransporter.prototype[key]) {
      isValid = false;
      if (!silent) {
        logger.error(`Invalid Transporter. Implementation missing for key :${key}`);
      }
    }
  });
  return isValid;
};
