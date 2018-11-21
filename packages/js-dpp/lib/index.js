const Ajv = require('ajv');

const DapObject = require('./dapObject/DapObject');
const DapContract = require('./dapContract/DapContract');
const STPacket = require('./stPacket/STPacket');
const STPacketHeader = require('./stPacket/STPacketHeader');

const SchemaValidator = require('./validation/SchemaValidator');

const validateDapObjectFactory = require('./dapObject/validateDapObjectFactory');
const validateDapContractFactory = require('./dapContract/validateDapContractFactory');
const validateStPacketFactory = require('./stPacket/validation/validateSTPacketFactory');
const validateStPacketStructureFactory = require('./stPacket/validation/validateSTPacketStructureFactory');

const validator = new SchemaValidator(new Ajv());

const validateDapContractStructure = validateDapContractFactory(validator);
const validateSTPacketStructure = validateStPacketStructureFactory(validator);

const enrichDapContractWithBaseDapObject = require('./dapObject/enrichDapContractWithBaseDapObject');

const validateDapObject = validateDapObjectFactory(validator, enrichDapContractWithBaseDapObject);
const validateSTPacket = validateStPacketFactory(
  validator,
  validateDapObject,
  validateDapContractStructure,
);

module.exports = {
  DapObject,
  DapContract,
  STPacket,
  STPacketHeader,
  factories,
  validateDapObject,
  validateSTPacket,
};
