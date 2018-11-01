const Ajv = require('ajv');

const validatePacketStructure = require('./stPacket/validatePacketStructure');
const validatePacketObjects = require('./stPacket/validatePacketObjects');

const dashSchema = require('../../../dash-schema/schema/schema');
const stPacketSchema = require('../../schema/st-packet');
const dapObjectSchema = require('../../schema/dap-object');
const appContractMetaSchema = require('../../schema/dap-contract');

module.exports = function validateStPacket(packet, appContract) {
  const ajv = new Ajv();

  ajv.addSchema(dashSchema);
  ajv.addSchema(stPacketSchema);
  ajv.addSchema(dapObjectSchema);
  ajv.addSchema(appContractMetaSchema);

  ajv.addSchema(appContract, 'dap-contract');

  let errors = validatePacketStructure(ajv, packet);

  if (errors) {
    return errors;
  }

  errors = validatePacketObjects(ajv, packet.objects);

  if (errors.length) {
    return errors;
  }

  return null;
};
