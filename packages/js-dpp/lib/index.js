const Ajv = require('ajv');

const DapObject = require('./dapObject/DapObject');
const DapContract = require('./dapContract/DapContract');
const STPacket = require('./stPacket/STPacket');

const SchemaValidator = require('./SchemaValidator');

const validateDapObjectFactory = require('./dapObject/validateDapObjectFactory');
const validateStPacketFactory = require('./stPacket/validation/validateSTPacketFactory');

const validateDapObjectStructureFactory = require('./dapObject/validateDapObjectStructureFactory');
const validateDapContractStructureFactory = require('./dapContract/validateDapContractStructureFactory');
const validateStPacketStructureFactory = require('./stPacket/validation/validateSTPacketStructureFactory');

const serializer = require('./serializer');

const hashingFunction = require('./hash');

const validator = new SchemaValidator(new Ajv());

const validateDapObjectStructure = validateDapObjectStructureFactory(validator);
const validateDapContractStructure = validateDapContractStructureFactory(validator);
const validateSTPacketStructure = validateStPacketStructureFactory(validator);

const enrichDapContractWithBaseDapObject = require('./dapObject/enrichDapContractWithBaseDapObject');

const validateDapObject = validateDapObjectFactory(validator, enrichDapContractWithBaseDapObject);
const validateSTPacket = validateStPacketFactory(
  validator,
  validateDapObject,
  validateDapContractStructure,
);

DapObject.setSerializer(serializer);
DapObject.setStructureValidator(validateDapObjectStructure);
DapObject.setHashingFunction(hashingFunction);

DapContract.setSerializer(serializer);
DapContract.setStructureValidator(validateDapContractStructure);
DapContract.setHashingFunction(hashingFunction);

STPacket.setSerializer(serializer);
STPacket.setStructureValidator(validateSTPacketStructure);
STPacket.setHashingFunction(hashingFunction);

module.exports = {
  DapObject,
  DapContract,
  STPacket,
  validateDapObject,
  validateSTPacket,
};
