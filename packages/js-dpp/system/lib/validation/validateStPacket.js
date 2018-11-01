const Ajv = require('ajv');

const dashSchema = require('../../../dash-schema/schema/schema');
const stPacketSchema = require('../../schema/st-packet');
const dapObjectSchema = require('../../schema/dap-object');
const appContractMetaSchema = require('../../schema/meta/dap-contract');

module.exports = function validateStPacket(packet, appContract) {
  const ajv = new Ajv();

  ajv.addMetaSchema(dashSchema);

  ajv.addSchema(stPacketSchema);
  ajv.addSchema(dapObjectSchema);
  ajv.addMetaSchema(appContractMetaSchema);

  ajv.addSchema(appContract, 'dap-contract');

  ajv.validate(
    'https://schema.dash.org/platform-4-0-0/system/st-packet',
    packet,
  );

  if (ajv.errors) {
    return ajv.errors;
  }

  const errors = [];
  for (const object of packet.objects) {
    try {
      ajv.validate({
        // eslint-disable-next-line no-underscore-dangle
        $ref: `dap-contract#/objects/${object._type}`,
      }, object);
    } catch (e) {
      if (e.missingSchema === 'dap-contract') {
        errors.push(e);

        // eslint-disable-next-line no-continue
        continue;
      }

      throw e;
    }

    if (ajv.errors) {
      errors.push(ajv.errors);
    }
  }

  if (errors.length) {
    return errors;
  }

  return null;
};
