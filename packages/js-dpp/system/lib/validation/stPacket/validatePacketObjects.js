function validatePacketObjects(ajv, objects) {
  const errors = [];

  for (const object of objects) {
    ajv.validate({
      $ref: `dap-contract#/objects/${object.type}`,
    }, object);

    if (ajv.errors) {
      errors.push(ajv.errors);
    }
  }

  return errors;
}

module.exports = validatePacketObjects;
