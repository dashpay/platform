const ReadOperation = require('./operations/ReadOperation');
const WriteOperation = require('./operations/WriteOperation');
const DeleteOperation = require('./operations/DeleteOperation');
const PreCalculatedOperation = require('./operations/PreCalculatedOperation');
const SignatureVerificationOperation = require('./operations/SignatureVerificationOperation');

const OPERATIONS = {
  read: ReadOperation,
  write: WriteOperation,
  delete: DeleteOperation,
  preCalculated: PreCalculatedOperation,
  signatureVerification: SignatureVerificationOperation,
};

function createOperationFromJSON(json) {
  const OperationClass = OPERATIONS[json.type];

  if (OperationClass) {
    throw new Error(`Operation ${json.type} is not supported`);
  }

  return OperationClass.fromJSON(json);
}

module.exports = createOperationFromJSON;
